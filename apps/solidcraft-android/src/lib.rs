//! SolidCraft Android launcher.
//!
//! Starts the app's egui UI from a `NativeActivity` through eframe/wgpu, and supplies the two
//! Android services a desktop app cannot have on its own:
//! - the soft keyboard (`ime_bridge` + the hidden `EditText` in `SolidCraftActivity`), so numeric
//!   fields, the command bar and dialogs can be typed into;
//! - file open/save through the Storage Access Framework (`saf_bridge` + the same Activity);
//! - the agent chat's API agents (`agent`): their keys kept on the device, HTTPS to the model
//!   endpoint, and the tools run in this process (Android cannot start a `solidcraft-cli`).
//!
//! Desktop-only host services (native dialogs, updater, SpaceMouse) stay absent.

mod agent;
mod finger_gestures;
mod ime_bridge;
mod keypad;
mod saf_bridge;

use saf_bridge::saf;
mod control_server;
mod system_fonts;

use android_activity::AndroidApp;
use egui::Modifiers;
use jni::{JavaVM, jni_sig, jni_str, objects::JObject};
use solidcraft_engine::Session;
use solidcraft_ui_egui::tablet_input;
use solidcraft_ui_egui::{Services, SolveApp};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use winit::platform::android::cad;

fn input_config(app: &AndroidApp) -> [i32; 3] {
    // SAFETY: AndroidApp owns the live VM and activity reference throughout this call.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    vm.attach_current_thread(|env| -> jni::errors::Result<[i32; 3]> {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
        let mut values = [8, 500, 300];
        for (index, value) in values.iter_mut().enumerate() {
            *value = env.call_method(&*activity, jni_str!("cadInputConfig"), jni_sig!("(I)I"), &[jni::objects::JValue::Int(index as i32)])?.i()?;
        }
        Ok(values)
    })
    .unwrap_or([8, 500, 300])
}

const ANDROID_SHELL_MARKER: &str = "SOLIDCRAFT_ANDROID_SHELL_V6_IME_SNAPSHOT";

/// Depth/stencil the egui pass allocates; the viewport callback is built for the same formats.
const DEPTH_BITS: u8 = 24;
const STENCIL_BITS: u8 = 8;
/// The tablet's Mali-G68 renders this CAD scene slowly with MSAA; keep it off until the port works.
const MSAA: u16 = 1;

// ---------------------------------------------------------------- SAF state and helpers

pub type SafInbox = ();

#[derive(Default)]
pub struct SafState {
    next_id: AtomicU64,
    mirror: saf::SaveMirror,
}

fn saf_next_id(state: &SafState) -> u64 {
    state.next_id.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
}

/// Open the system picker and block until the user answers. The picker runs on the Java UI thread
/// while `android_main` runs on its own Rust thread, so waiting does not freeze the UI.
fn saf_open_paths(bridge: &saf::Bridge, state: &SafState) -> Vec<String> {
    let id = saf_next_id(state);
    match bridge.pick_blocking(id, None, saf::PICK_TIMEOUT) {
        Ok(saf::PickAnswer::Paths(paths)) => paths,
        Ok(saf::PickAnswer::Cancelled) => Vec::new(),
        Err(error) => {
            log::error!("Android open picker: {error}");
            Vec::new()
        }
    }
}

/// Save dialog: the staged app-private destination, watched so the app's own write is mirrored to
/// the chosen SAF document on the following frames.
fn saf_save_path(bridge: &saf::Bridge, state: &SafState, name: &str) -> Option<String> {
    let id = saf_next_id(state);
    match bridge.pick_blocking(id, Some(name), saf::PICK_TIMEOUT) {
        Ok(saf::PickAnswer::Paths(paths)) => {
            let path = paths.into_iter().next();
            if let Some(path) = &path {
                state.mirror.track(id, path);
            }
            path
        }
        Ok(saf::PickAnswer::Cancelled) => None,
        Err(error) => {
            log::error!("Android save picker: {error}");
            None
        }
    }
}

fn saf_poll(bridge: &saf::Bridge, state: &SafState) {
    state.mirror.sync(bridge);
}

fn services(bridge: saf::Bridge, state: Arc<SafState>, files: Option<std::path::PathBuf>) -> Services {
    let open_bridge = bridge.clone_ref();
    let open_state = Arc::clone(&state);
    let save_bridge = bridge.clone_ref();
    let save_state = Arc::clone(&state);
    let mut services = Services {
        pick_open: Some(Box::new(move |_exts: &[&str]| {
            let paths = saf_open_paths(&open_bridge, &open_state);
            Box::pin(async move { paths.into_iter().next() })
        })),
        pick_save: Some(Box::new(move |name: &str, _exts: &[&str]| {
            let path = saf_save_path(&save_bridge, &save_state, name);
            Box::pin(async move { path })
        })),
        ..Services::default()
    };
    // The AI panel's API agents: HTTPS to the endpoint, the key kept on this device.
    services.agent_http = Some(agent::http());
    if let Some(dir) = files {
        services.secrets = Some(agent::FileSecrets::new(&dir));
    }
    services
}

// ---------------------------------------------------------------- Android IME visibility

/// Show or hide the soft keyboard through the Activity's `imeShow`/`imeHide`, which own the hidden
/// `EditText` the characters come from.
fn ime_visibility(app: &AndroidApp, show: bool) {
    let app_clone = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        // SAFETY: AndroidApp owns the live VM pointer for the lifetime of the activity.
        let jvm = unsafe { JavaVM::from_raw(app_clone.vm_as_ptr() as *mut _) };
        let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let activity_raw = app_clone.activity_as_ptr() as jni::sys::jobject;
            // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it.
            let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
            if show {
                env.call_method(&*activity, jni_str!("imeShow"), jni_sig!("()V"), &[])?;
            } else {
                env.call_method(&*activity, jni_str!("imeHide"), jni_sig!("()V"), &[])?;
            }
            Ok(())
        });
        if let Err(error) = result {
            log::error!("Android IME visibility: {error}");
        }
    }));
}

/// Finish the Activity (its `finish()`), so a window that went away shows the launcher instead of a
/// black screen.
fn finish_activity(app: &AndroidApp) {
    let app_clone = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        // SAFETY: AndroidApp owns the live VM pointer for the lifetime of the activity.
        let jvm = unsafe { JavaVM::from_raw(app_clone.vm_as_ptr() as *mut _) };
        let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let activity_raw = app_clone.activity_as_ptr() as jni::sys::jobject;
            // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it.
            let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
            env.call_method(&*activity, jni_str!("finish"), jni_sig!("()V"), &[])?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("Android finish(): {error}");
        }
    }));
}

/// Reconstruct egui's modifier state on Android.
///
/// winit's Android backend delivers `KeyboardInput` events but never emits `ModifiersChanged`
/// (it discards the `KeyEvent`'s meta state), so egui's modifier state stays empty and
/// `Ctrl`/`Alt` shortcuts are ignored while plain keys continue to work. Physical modifier keys
/// do arrive as `Event::Key` events, so we track them and stamp the resulting state onto this
/// frame's events, suppressing the text character that would otherwise be emitted alongside a
/// Ctrl/Command shortcut.
fn apply_android_modifiers(state: &mut egui::Modifiers, raw: &mut egui::RawInput) {
    let before = *state;
    let mut kept: Vec<egui::Event> = Vec::with_capacity(raw.events.len());
    for mut event in raw.events.drain(..) {
        match &mut event {
            egui::Event::Key { key, physical_key, pressed, modifiers, .. } => {
                let tracked = physical_key.unwrap_or(*key);
                match tracked {
                    egui::Key::ShiftLeft | egui::Key::ShiftRight => state.shift = *pressed,
                    egui::Key::ControlLeft | egui::Key::ControlRight => state.ctrl = *pressed,
                    egui::Key::AltLeft | egui::Key::AltRight => state.alt = *pressed,
                    _ => {}
                }
                state.mac_cmd = false;
                state.command = state.ctrl;
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::PointerButton { modifiers, .. } => {
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::MouseWheel { modifiers, .. } => {
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::Text(_) if state.ctrl || state.command || state.mac_cmd => {}
            egui::Event::WindowFocused(false) => {
                *state = egui::Modifiers::default();
                kept.push(event);
            }
            _ => kept.push(event),
        }
    }
    raw.events = kept;
    if *state != before {
        raw.events.push(egui::Event::ModifiersChanged(*state));
    }
}

// ---------------------------------------------------------------- runtime switches

/// Read an `env.txt` (`KEY=VALUE` per line) and set those variables, so a renderer, a backend or the
/// control server can be switched on the device without rebuilding the APK.
///
/// The app's own data dir is not writable from `adb push` (a release build is not debuggable), so
/// its external files dir is read too.
fn apply_env_overrides(dir: &std::path::Path) {
    // Android's own temp is /data/local/tmp, which an app cannot write: screenshots and the MCP
    // server's scratch folder need one of the app's own instead.
    if std::env::var_os("TMPDIR").is_none() {
        let tmp = dir.join("tmp");
        if std::fs::create_dir_all(&tmp).is_ok() {
            // SAFETY: called before any thread is spawned, before eframe reads the environment.
            unsafe { std::env::set_var("TMPDIR", &tmp) };
            log::info!("env override: TMPDIR={}", tmp.display());
        }
    }
    let external = std::path::Path::new("/sdcard/Android/data/ai.storyteller.solidcraft/files");
    let text = std::fs::read_to_string(dir.join("env.txt")).or_else(|_| std::fs::read_to_string(external.join("env.txt"))).ok();
    let Some(text) = text else { return };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            let shown = if key.contains("TOKEN") { "<hidden>" } else { value };
            log::info!("env override: {key}={shown}");
            // SAFETY: called before any thread is spawned and before eframe reads the environment.
            unsafe { std::env::set_var(key, value) };
        }
    }
}

// ---------------------------------------------------------------- shell

struct Shell {
    off_hand: std::collections::BTreeMap<(i32, i32), (egui::accesskit::NodeId, egui::accesskit::Action, [f32; 2])>,
    pen_transaction: tablet_input::Transaction,
    dock: tablet_input::Dock,
    input_config: [i32; 3],
    input_trace: String,
    keypad: keypad::Keypad,
    gestures: finger_gestures::Gestures,
    solve: SolveApp,
    app: AndroidApp,
    bridge: saf::Bridge,
    state: Arc<SafState>,
    ime: ime_bridge::ime::ImeBridge,
    modifiers: Modifiers,
    keyboard_wanted: bool,
    /// The widget that had the keyboard last frame. Moving to another field starts the IME's buffer
    /// from scratch, so the bridge must forget what the old field already received.
    keyboard_owner: Option<egui::Id>,
}

impl eframe::App for Shell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(contact) = cad::last_contact() {
            self.input_trace = format!(
                "{:?} {}:{} {:?} pressure={:.2} buttons={} time={} {}",
                contact.source,
                contact.device,
                contact.id,
                contact.phase,
                contact.pressure,
                contact.buttons,
                contact.time_ns,
                if matches!(contact.source, cad::Source::Stylus | cad::Source::Eraser) { "PenDraw" } else { "Panel/Idle" }
            );
        }
        let mut queued = Vec::new();
        for contact in winit::platform::android::cad::drain() {
            self.input_trace = format!("{:?} {}:{} {:?} Viewport", contact.source, contact.device, contact.id, contact.phase);
            if let Some((delta, pan, zoom)) = self.gestures.event(contact) {
                self.input_trace.push_str(&format!(" {} delta={delta:?} zoom={zoom:.3}", if pan { "FingerPanZoom" } else { "FingerOrbit" }));
                queued.push((delta, pan, zoom));
            }
        }
        let scale = ctx.pixels_per_point();
        let anchor = self.gestures.anchor().map(|[x, y]| [x / scale, y / scale]);
        for (delta, pan, zoom) in finger_gestures::coalesce(queued) {
            solidcraft_ui_egui::navigation::tablet_motion(&mut self.solve, [delta[0] / scale, delta[1] / scale], pan, zoom, anchor);
        }
        self.input_trace.push_str(&format!(" contacts={}", self.gestures.contact_count()));
        self.solve.logic(ctx);
        saf_poll(&self.bridge, &self.state);
        // The soft keyboard follows egui's focus: any focused TextEdit wants it.
        let wanted = ctx.egui_wants_keyboard_input();
        let owner = if wanted { ctx.memory(|m| m.focused()) } else { None };
        if wanted != self.keyboard_wanted {
            self.keyboard_wanted = wanted;
            // CAD text fields use the in-app keypad. The Android IME remains hidden by
            // default; this also prevents focus changes from repeatedly popping it up.
            if !wanted {
                ime_visibility(&self.app, false);
            }
            self.ime.reset();
        } else if owner != self.keyboard_owner {
            self.ime.reset();
        }
        self.keyboard_owner = owner;
        if self.solve.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let context =
            self.solve.tool.as_ref().map(|tool| tool.cmd.to_owned()).unwrap_or_else(|| {
                self.solve.dialog.as_ref().map(|dialog| format!("{:?}", std::mem::discriminant(&dialog.kind))).unwrap_or_default()
            });
        self.dock.context(format!("{:?}:{context}", self.solve.session.active_sketch));
        let slop = (self.input_config[0] as f32 / ctx.pixels_per_point()).clamp(2.0, 64.0);
        let hold = (self.input_config[1] as f64 / 1000.0).clamp(0.2, 2.0);
        ctx.options_mut(|options| {
            options.input_options.max_click_dist = slop;
            options.input_options.max_double_click_delay = (self.input_config[2] as f64 / 1000.0).clamp(0.15, 1.0);
        });
        for event in cad::drain_controls() {
            let c = event.contact;
            let at = [c.position[0] / ctx.pixels_per_point(), c.position[1] / ctx.pixels_per_point()];
            if event.control == 254 {
                let key = (c.device, c.id);
                if c.phase == cad::Phase::Down {
                    if let Some((id, action)) = tablet_input::off_hand_target(ctx, at) {
                        self.off_hand.insert(key, (id, action, at));
                    }
                } else if matches!(c.phase, cad::Phase::Up | cad::Phase::Cancel) {
                    if let Some((id, action, origin)) = self.off_hand.remove(&key) {
                        if c.phase == cad::Phase::Up && (at[0] - origin[0]).hypot(at[1] - origin[1]) <= slop {
                            raw.events.push(tablet_input::off_hand_action(id, action));
                        }
                    }
                }
                continue;
            }
            if c.phase == cad::Phase::Move {
                self.dock.move_contact(c.device, c.id, at, matches!(c.source, cad::Source::Stylus | cad::Source::Eraser), slop);
            } else if matches!(c.phase, cad::Phase::Down | cad::Phase::Up | cad::Phase::Cancel) {
                self.dock.contact(
                    c.device,
                    c.id,
                    event.control,
                    c.phase == cad::Phase::Down,
                    c.phase == cad::Phase::Cancel,
                    [c.position[0] / ctx.pixels_per_point(), c.position[1] / ctx.pixels_per_point()],
                    c.time_ns as f64 / 1e9,
                    hold,
                    slop,
                );
            }
        }
        tablet_input::set(tablet_input::Policy {
            enabled: true,
            pen: matches!(cad::pointer_source(), cad::Source::Stylus | cad::Source::Eraser),
            hold_seconds: hold,
            slop,
            actions: self.dock.actions(),
            filter: self.dock.filter,
        });
        if tablet_input::pen()
            && raw.events.iter().any(|e| {
                matches!(e, egui::Event::PointerButton { pos, pressed: true, button: egui::PointerButton::Primary, .. }
            if self.solve.viewport.rect.is_some_and(|r| r.contains(*pos)))
            })
        {
            self.pen_transaction.begin(&mut self.solve);
        }
        self.keypad.intercept_pointer(ctx, raw);
        self.keypad.restore_focus(ctx);
        if cad::take_pen_cancel() {
            self.off_hand.clear();
            self.dock.reset();
            tablet_input::set(tablet_input::Policy { enabled: true, ..Default::default() });
            self.pen_transaction.cancel(&mut self.solve);
            // Cancel the active interaction before a canceled pen can be interpreted as a click.
            raw.events.retain(|event| !matches!(event, egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. }));
            let modifiers = egui::Modifiers::default();
            raw.events.push(egui::Event::PointerMoved(egui::pos2(-100000.0, -100000.0)));
            raw.events.push(egui::Event::PointerButton {
                pos: egui::pos2(-100000.0, -100000.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers,
            });
            raw.events.push(egui::Event::PointerGone);
        }
        if !raw.focused {
            self.off_hand.clear();
            self.dock.reset();
            tablet_input::set(tablet_input::Policy { enabled: true, ..Default::default() });
            self.pen_transaction.cancel(&mut self.solve);
            self.keypad.reset_focus();
            self.modifiers = Modifiers::default();
            self.gestures.reset();
            self.keypad.shift = false;
            self.keypad.ctrl = false;
            winit::platform::android::cad::reset();
        }
        apply_android_modifiers(&mut self.modifiers, raw);
        self.keypad.input(raw, self.modifiers);
        for action in self.dock.take_actions() {
            match action {
                22 | 23 => raw.events.extend(tablet_input::key_events(if action == 22 { "Tab" } else { "Delete" }).unwrap_or_default()),
                40..=46 => {
                    if let Some(shortcut) = self.dock.shortcuts.get((action - 40) as usize) {
                        if let Some(events) = tablet_input::key_events(shortcut) {
                            // Explicit shortcut modifiers belong to this command, independent of held CTRL.
                            raw.events.push(egui::Event::ModifiersChanged(
                                events
                                    .first()
                                    .and_then(|e| if let egui::Event::Key { modifiers, .. } = e { Some(*modifiers) } else { None })
                                    .unwrap_or_default(),
                            ));
                            raw.events.extend(events);
                        } else {
                            self.solve.set_status("Open QP → Edit slots to assign a valid shortcut.", false);
                        }
                    }
                }
                _ => {
                    if action == 21 {
                        self.pen_transaction.cancel(&mut self.solve);
                    } else {
                        self.pen_transaction.complete(&mut self.solve);
                    }
                    tablet_input::panel_action(&mut self.solve, action);
                    if action == 21 {
                        cad::cancel_modeling();
                        ctx.request_repaint();
                    }
                }
            }
        }
        self.ime.poll(raw);
        self.solve.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        system_fonts::install_system_fonts(ui.ctx());
        self.keypad.show(ui);
        self.solve.ui(ui);
        if !ui.input(|i| i.pointer.primary_down()) {
            self.pen_transaction.complete(&mut self.solve);
        }
        let scale = ui.ctx().pixels_per_point();
        let regions = if self.solve.viewport.rect.is_some() {
            self.dock.show(ui.ctx(), self.solve.session.active_sketch.is_some(), self.solve.dialog.is_some(), self.solve.viewport.boxsel.is_some())
        } else {
            Vec::new()
        };
        cad::controls(regions.into_iter().map(|(id, r)| (id, [r.min.x * scale, r.min.y * scale, r.max.x * scale, r.max.y * scale])).collect());
        if cfg!(debug_assertions) {
            egui::Area::new(egui::Id::new("android-input-trace")).anchor(egui::Align2::RIGHT_BOTTOM, [-8.0, -8.0]).interactable(false).show(
                ui.ctx(),
                |ui| {
                    ui.label(&self.input_trace);
                },
            );
        }
        let scale = ui.ctx().pixels_per_point();
        cad::exclude(
            solidcraft_ui_egui::navigation::tablet_exclusions(&self.solve)
                .into_iter()
                .map(|r| [r.min.x * scale, r.min.y * scale, r.max.x * scale, r.max.y * scale])
                .collect(),
        );
        winit::platform::android::cad::viewport(
            self.solve.viewport.rect.map(|r| [r.min.x * scale, r.min.y * scale, r.max.x * scale, r.max.y * scale]),
        );
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(settings) = serde_json::to_string(&self.dock) {
            storage.set_string("android-cad-panel", settings);
        }
        if let Ok(settings) = serde_json::to_string(&self.keypad) {
            storage.set_string("android-cad-keypad", settings);
        }
        storage.set_string(solidcraft_ui_egui::prefs::STORAGE_KEY, self.solve.prefs());
    }

    fn on_exit(&mut self) {
        solidcraft_ui_egui::agent_chat::shutdown(&mut self.solve);
        solidcraft_ui_egui::recovery_ui::close(&mut self.solve);
        solidcraft_engine::crash::end_session();
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let cad_config = input_config(&app);
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("SolidCraft"));
    log::info!("{ANDROID_SHELL_MARKER}");
    system_fonts::install_font_env(&app);
    let bridge = saf::Bridge::new(&app);
    let files = bridge.files_dir().ok();
    if let Some(dir) = files.as_deref() {
        apply_env_overrides(dir);
    }
    let state: Arc<SafState> = Arc::new(SafState::default());
    let ime = ime_bridge::ime::ImeBridge::new(&app);
    let shell_app = app.clone();
    let app_finish = app.clone();
    let shell_bridge = bridge.clone_ref();
    let shell_state = Arc::clone(&state);
    let shell_files = files.clone();
    solidcraft_engine::crash::phase("window");
    let options = eframe::NativeOptions {
        depth_buffer: DEPTH_BITS,
        stencil_buffer: STENCIL_BITS,
        multisampling: MSAA,
        android_app: Some(app),
        // eframe finds no storage folder on Android (it gives up on the OS), so the preferences
        // were never kept: the AI agents the user adds, the units, the permissions mode all went
        // away with the process. A file of the app's own fixes that.
        persistence_path: files.as_ref().map(|d| d.join("app.ron")),
        ..Default::default()
    };
    let result = eframe::run_native(
        "SolidCraft",
        options,
        Box::new(move |cc| {
            tablet_input::install(&cc.egui_ctx);
            let hit_ctx = cc.egui_ctx.clone();
            cad::hit_test(Box::new(move |[x, y]| {
                let pos = egui::pos2(x, y) / hit_ctx.pixels_per_point();
                hit_ctx.layer_id_at(pos).is_none_or(|layer| layer.order == egui::Order::Background)
            }));
            solidcraft_engine::crash::phase("app setup");
            solidcraft_ui_egui::fonts::setup_fonts(&cc.egui_ctx);
            let mut app_ui = SolveApp::new(Session::default(), services(shell_bridge.clone_ref(), Arc::clone(&shell_state), shell_files.clone()));
            // The preferences the last run saved (agents, units, permissions…): without this the
            // app started from the defaults every time, since eframe only hands the file over.
            if let Some(p) = cc.storage.and_then(|s| solidcraft_ui_egui::prefs::stored(|k| s.get_string(k))) {
                app_ui.load_prefs(&p);
            }
            // The chat's own loopback endpoint: the agent's tool calls land in the gate there, and
            // its channel is what lets the tools run in this process (there is no CLI to start).
            {
                let ctx = cc.egui_ctx.clone();
                app_ui.agent_chat.control_starter = Some(Box::new(move |tx| control_server::listen_agent(tx, ctx.clone())));
            }
            app_ui.apply_off_thread = Some(solidcraft_ui_egui::dialogs::APPLY_AT_ONCE);
            // The viewport draws through an egui_wgpu paint callback, so the app needs the render
            // state: without this the UI works but the 3D scene (grid, origin planes, bodies) is
            // never painted and nothing can be selected.
            if let Some(rs) = &cc.wgpu_render_state {
                log::info!("graphics: {:?}, MSAA {}x", rs.adapter.get_info().backend, rs.msaa_samples);
                app_ui.set_wgpu(rs, DEPTH_BITS, STENCIL_BITS, rs.msaa_samples);
                solidcraft_ui_egui::gpu::install_picture_renderer(rs);
            } else {
                log::error!("no wgpu render state: the 3D viewport cannot be painted");
            }
            // Optional loopback control server, so tests and agents can click by name instead of
            // guessing screen coordinates (`adb forward tcp:9977 tcp:9977`, JSON lines).
            if let Some(port) = std::env::var("SOLIDCRAFT_CONTROL_PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
                let rx = control_server::start(port, cc.egui_ctx.clone());
                app_ui = app_ui.with_control(rx);
                log::info!("control server listening on 127.0.0.1:{port}");
            }
            Ok(Box::new(Shell {
                off_hand: Default::default(),
                pen_transaction: tablet_input::Transaction::default(),
                dock: cc.storage.and_then(|s| s.get_string("android-cad-panel")).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                input_config: cad_config,
                input_trace: "Input diagnostics: idle".into(),
                keypad: cc.storage.and_then(|s| s.get_string("android-cad-keypad")).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                gestures: finger_gestures::Gestures::default(),
                solve: app_ui,
                app: shell_app,
                bridge: shell_bridge,
                state: shell_state,
                ime,
                modifiers: Modifiers::default(),
                keyboard_wanted: false,
                keyboard_owner: None,
            }))
        }),
    );
    if let Err(error) = result {
        log::error!("SolidCraft could not start: {error}");
    }
    // eframe returning means the window is gone: on Android the activity would stay alive with no
    // surface (a black screen), so log it and finish the activity instead.
    log::info!("eframe event loop returned; finishing the activity");
    finish_activity(&app_finish);
}
