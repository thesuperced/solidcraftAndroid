//! Tablet selection policy and off-hand controls sharing the desktop command paths.
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub enabled: bool,
    pub pen: bool,
    pub hold_seconds: f64,
    pub slop: f32,
    pub actions: [bool; 7],
    pub filter: Filter,
}
impl Default for Policy {
    fn default() -> Self {
        Self { enabled: false, pen: false, hold_seconds: 0.5, slop: 8.0, actions: [false; 7], filter: Filter::All }
    }
}
thread_local! { static POLICY: RefCell<Policy> = RefCell::new(Policy::default()); }
pub fn set(policy: Policy) {
    POLICY.with(|p| *p.borrow_mut() = policy);
}
pub fn policy() -> Policy {
    POLICY.with(|p| *p.borrow())
}
pub fn pen() -> bool {
    let p = policy();
    p.enabled && p.pen
}
pub fn action(index: usize) -> bool {
    let p = policy();
    p.enabled && p.actions.get(index).copied().unwrap_or(false)
}
pub const ADD: usize = 0;
pub const RANGE: usize = 1;
pub const NO_SNAP: usize = 2;
pub const MIDPOINT: usize = 3;
pub const FREE: usize = 4;
pub const RESELECT: usize = 5;
pub const COPY: usize = 6;

#[derive(Default)]
struct UiTargets {
    nodes: Vec<(egui::accesskit::NodeId, egui::accesskit::Node)>,
}
impl egui::Plugin for UiTargets {
    fn debug_name(&self) -> &'static str {
        "Tablet UI targets"
    }
    fn output_hook(&mut self, _ctx: &egui::Context, output: &mut egui::FullOutput) {
        self.nodes = output.platform_output.accesskit_update.as_ref().map(|tree| tree.nodes.clone()).unwrap_or_default();
    }
}
pub fn install(ctx: &egui::Context) {
    ctx.enable_accesskit();
    ctx.add_plugin(UiTargets::default());
}

/// Click/focus an off-hand UI target through egui's accessibility action path. This
/// activates its existing widget without borrowing the pen's single pointer cursor.
pub fn off_hand_target(ctx: &egui::Context, at: [f32; 2]) -> Option<(egui::accesskit::NodeId, egui::accesskit::Action)> {
    use egui::accesskit::Action;
    {
        let targets = ctx.plugin_or_default::<UiTargets>();
        let targets = targets.lock();
        targets
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                let bounds = node.bounds()?;
                let contains =
                    (at[0] as f64) >= bounds.x0 && (at[0] as f64) <= bounds.x1 && (at[1] as f64) >= bounds.y0 && (at[1] as f64) <= bounds.y1;
                let action = if node.supports_action(Action::Click) {
                    Action::Click
                } else if node.supports_action(Action::Focus) {
                    Action::Focus
                } else {
                    return None;
                };
                contains.then_some((*id, action, (bounds.x1 - bounds.x0) * (bounds.y1 - bounds.y0)))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, action, _)| (id, action))
    }
}
pub fn off_hand_action(id: egui::accesskit::NodeId, action: egui::accesskit::Action) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action,
        target_node: id,
        target_tree: egui::accesskit::TreeId::ROOT,
        data: None,
    })
}

/// One native pen contact is one cancellable modeling transaction. Checkpoints are shared
/// engine commands/state, so cancellation restores the design and undo/redo stacks together.
#[derive(Default)]
pub struct Transaction {
    checkpoint: Option<u64>,
    selection: Vec<solidcraft_engine::Sel>,
}
impl Transaction {
    pub fn begin(&mut self, app: &mut crate::SolveApp) {
        if self.checkpoint.is_none() {
            self.selection = app.session.selection.clone();
            self.checkpoint = Some(app.session.checkpoint());
        }
    }
    pub fn complete(&mut self, app: &mut crate::SolveApp) {
        if let Some(id) = self.checkpoint.take() {
            app.session.drop_checkpoint(id);
        }
        self.selection.clear();
    }
    pub fn cancel(&mut self, app: &mut crate::SolveApp) {
        cancel(app);
        if let Some(id) = self.checkpoint.take() {
            if let Err(error) = app.session.restore_checkpoint(id) {
                app.set_status(error.to_string(), true);
            }
            app.session.selection = std::mem::take(&mut self.selection);
        }
    }
}
pub fn cancel(app: &mut crate::SolveApp) {
    crate::sketch_text_handles::drag_end(app);
    crate::sketch_text_move::cancel_drag(app);
    app.viewport.point_drag = None;
    app.viewport.boxsel = None;
    app.viewport.draw_drag = None;
    app.viewport.tool_click = false;
    app.viewport.trim_drag = false;
    app.viewport.joint_drag = None;
    crate::drag_snap::clear();
    crate::dim_view::cancel();
    crate::dim_view::drag_end();
    crate::sketch3d::drag_end();
    crate::select_other::close(app);
    crate::tool_modifiers::set_manipulator_dragging(false);
    app.tool = None;
    if app.dialog.is_some() {
        crate::dialogs::cancel(app);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Filter {
    #[default]
    All,
    Bodies,
    Faces,
    Edges,
    Vertices,
    Sketch,
}
impl Filter {
    pub const ALL: [Self; 6] = [Self::All, Self::Bodies, Self::Faces, Self::Edges, Self::Vertices, Self::Sketch];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Bodies => "Bodies",
            Self::Faces => "Faces",
            Self::Edges => "Edges",
            Self::Vertices => "Vertices",
            Self::Sketch => "Sketch",
        }
    }
    pub fn accepts(self, hit: &crate::viewport::Hit) -> bool {
        use crate::viewport::Hit;
        match self {
            Self::All => true,
            Self::Bodies | Self::Faces => matches!(hit, Hit::Face { .. }),
            Self::Edges => matches!(hit, Hit::Edge { .. }),
            Self::Vertices => matches!(hit, Hit::Vertex { .. }),
            Self::Sketch => matches!(hit, Hit::SketchPoint { .. } | Hit::SketchCurve { .. } | Hit::Profile { .. }),
        }
    }
}

/// Native contacts keep their own capture so controls never borrow the pen cursor.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Dock {
    #[serde(skip)]
    held: BTreeMap<(i32, i32), (u8, f64, [f32; 2])>,
    #[serde(skip)]
    pub filter: Filter,
    pub right: bool,
    position: Option<[f32; 2]>,
    hidden: bool,
    quick_open: bool,
    pub shortcuts: [String; 7],
    #[serde(skip)]
    editing: bool,
    #[serde(skip)]
    context: String,
    #[serde(skip)]
    pending: Vec<u8>,
    #[serde(skip)]
    drag: Option<([f32; 2], [f32; 2])>,
    #[serde(skip)]
    dragging: bool,
    #[serde(skip)]
    panel_origin: [f32; 2],
}
impl Dock {
    pub fn reset(&mut self) {
        self.held.clear();
        self.drag = None;
        self.dragging = false;
        self.pending.clear();
    }
    pub fn context(&mut self, key: String) {
        if self.context != key {
            // Filters belong to the current modeling context, never leak from faces to sketches.
            self.filter = Filter::All;
            self.context = key;
        }
    }
    pub fn actions(&self) -> [bool; 7] {
        let mut active = [false; 7];
        active[ADD] = self.held.values().any(|(id, _, _)| *id == 0);
        active
    }
    pub fn take_actions(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn activate(&mut self, id: u8) {
        match id {
            0 => {}
            20 => {
                self.right = !self.right;
                self.position = None;
            }
            25 => self.quick_open = !self.quick_open,
            28 => self.hidden = true,
            30 => self.hidden = false,
            47 => self.editing = !self.editing,
            10..=15 => {
                if let Some(filter) = id.checked_sub(10).and_then(|i| Filter::ALL.get(i as usize)) {
                    self.filter = *filter;
                }
            }
            _ => self.pending.push(id),
        }
    }
    pub fn move_contact(&mut self, device: i32, pointer: i32, at: [f32; 2], pen: bool, slop: f32) {
        if !pen || self.held.get(&(device, pointer)).is_none_or(|(id, _, _)| *id != 20) {
            return;
        }
        if let Some((origin, base)) = self.drag {
            let delta = [at[0] - origin[0], at[1] - origin[1]];
            if self.dragging || delta[0].hypot(delta[1]) > slop {
                self.dragging = true;
                self.position = Some([base[0] + delta[0], base[1] + delta[1]]);
            }
        }
    }
    /// Ctrl is strictly momentary, like a physical keyboard key; every Up/Cancel releases it.
    pub fn contact(&mut self, device: i32, pointer: i32, control: u8, down: bool, cancel: bool, at: [f32; 2], now: f64, hold: f64, slop: f32) {
        let key = (device, pointer);
        if down {
            self.held.entry(key).or_insert((control, now, at));
            if control == 20 {
                self.drag = Some((at, self.panel_origin));
                self.dragging = false;
            }
        } else if let Some((id, start, origin)) = self.held.remove(&key) {
            if id == 20 {
                self.drag = None;
                if std::mem::take(&mut self.dragging) {
                    return;
                }
            }
            if cancel || id == 0 {
                return;
            }
            if now >= start && (at[0] - origin[0]).hypot(at[1] - origin[1]) <= slop {
                if (40..=46).contains(&id) && now - start >= hold {
                    self.editing = true;
                } else {
                    self.activate(id);
                }
            }
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, _sketch: bool, _feature: bool, selecting_box: bool) -> Vec<(u8, egui::Rect)> {
        let mut regions = Vec::new();
        let screen = ctx.content_rect();
        if self.hidden {
            let out = egui::Area::new(egui::Id::new("tablet_restore"))
                .fixed_pos(egui::pos2(screen.left(), screen.center().y))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let r = ui.add_sized([28.0, 64.0], egui::Button::new("›"));
                    regions.push((30, r.rect));
                    r.clicked()
                });
            if out.inner {
                self.activate(30);
            }
            return regions;
        }
        let height = 9.0 * 44.0 + 22.0 + 48.0;
        let extra_width = if self.quick_open { 104.0 } else { 0.0 } + if selecting_box { 104.0 } else { 0.0 };
        let default =
            egui::pos2(if self.right { screen.right() - 110.0 } else { screen.left() + 12.0 }, (screen.center().y - height * 0.5).max(screen.top()));
        let p = self.position.map(|p| egui::pos2(p[0], p[1])).unwrap_or(default);
        let p = egui::pos2(
            p.x.clamp(screen.left(), (screen.right() - 110.0 - extra_width).max(screen.left())),
            p.y.clamp(screen.top(), (screen.bottom() - height).max(screen.top())),
        );
        self.panel_origin = [p.x, p.y];
        let mut clicked = Vec::new();
        let out = egui::Area::new(egui::Id::new("tablet_semantic_dock")).fixed_pos(p).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (id, label, h) in [
                    (20, if self.right { "←" } else { "→" }, 22.0),
                    (0, "CTRL", 44.0),
                    (21, "Cancel", 44.0),
                    (22, "TAB", 44.0),
                    (23, "Delete", 44.0),
                    (25, "QP", 44.0),
                    (26, "Redo", 44.0),
                    (27, "Undo", 44.0),
                    (28, "Hide", 44.0),
                    (29, "Finish", 44.0),
                ] {
                    let r =
                        ui.add_sized([82.0, h], egui::Button::new(label).selected((id == 0 && self.actions()[ADD]) || (id == 25 && self.quick_open)));
                    regions.push((id, r.rect));
                    if r.clicked() {
                        clicked.push(id);
                    }
                    crate::scenario::publish_handle(&format!("tablet:{label}"), r.rect.center());
                }
            });
        });
        // Capture panel gaps too, preventing accidental canvas navigation behind the panel.
        regions.push((253, out.response.rect));
        let extra_x = (out.response.rect.right() + 6.0).min((screen.right() - 110.0).max(screen.left()));
        if self.quick_open {
            egui::Area::new(egui::Id::new("tablet_quick_panel")).fixed_pos(egui::pos2(extra_x, p.y)).order(egui::Order::Foreground).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (i, shortcut) in self.shortcuts.iter().enumerate() {
                        let label = if shortcut.is_empty() { format!("Slot {}", i + 1) } else { shortcut.clone() };
                        let r = ui.add_sized([82.0, 44.0], egui::Button::new(label));
                        regions.push((40 + i as u8, r.rect));
                        if r.clicked() {
                            clicked.push(40 + i as u8);
                        }
                    }
                    let r = ui.add_sized([82.0, 32.0], egui::Button::new("Edit slots"));
                    regions.push((47, r.rect));
                    if r.clicked() {
                        clicked.push(47);
                    }
                });
            });
        }
        if selecting_box {
            // Keep filters available beyond the pen cursor, without stealing its capture.
            egui::Area::new(egui::Id::new("tablet_box_filters"))
                .fixed_pos(egui::pos2(if self.quick_open { (extra_x + 104.0).min(screen.right() - 100.0) } else { extra_x }, p.y))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        for (i, filter) in Filter::ALL.iter().enumerate() {
                            let r = ui.add_sized([82.0, 44.0], egui::Button::new(filter.label()).selected(self.filter == *filter));
                            regions.push((10 + i as u8, r.rect));
                            if r.clicked() {
                                clicked.push(10 + i as u8);
                            }
                        }
                    });
                });
        }
        for id in clicked {
            self.activate(id);
        }
        if self.editing {
            egui::Window::new("Quick Panel shortcuts").collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label("Enter shortcuts such as M, E, or Ctrl+Shift+K.");
                for (i, shortcut) in self.shortcuts.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Slot {}", i + 1));
                        ui.add(egui::TextEdit::singleline(shortcut).char_limit(40).desired_width(140.0));
                    });
                }
                if ui.button("Done").clicked() {
                    self.editing = false;
                }
            });
        }
        regions
    }
}

/// Run the existing key paths. This also handles focus traversal in active dimension fields.
pub fn key_events(shortcut: &str) -> Option<Vec<egui::Event>> {
    let mut modifiers = egui::Modifiers::NONE;
    let mut key = None;
    for part in shortcut.trim().split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" => {
                modifiers.ctrl = true;
                modifiers.command = true;
            }
            "shift" => modifiers.shift = true,
            "alt" => modifiers.alt = true,
            name if key.is_none() => {
                key = egui::Key::from_name(&name.to_ascii_uppercase()).or_else(|| egui::Key::from_name(part.trim()));
                key?;
            }
            _ => return None,
        }
    }
    let key = key?;
    Some([true, false].into_iter().map(|pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }).collect())
}
/// Commands with field-sensitive behavior run before the normal widget/shortcut pass.
pub fn panel_action(app: &mut crate::SolveApp, action: u8) {
    match action {
        21 => {
            cancel(app);
            let _ = app.run("select.clear", serde_json::json!({}));
        }
        26 => {
            let _ = app.run("edit.redo", serde_json::json!({}));
        }
        27 => {
            let _ = app.run("edit.undo", serde_json::json!({}));
        }
        29 => {
            if app.dialog.is_some() {
                app.synthetic.extend(key_events("Enter").unwrap_or_default());
            } else if app.tool.is_some() {
                crate::tools::finish(app);
                // Finish ends the tool, including a line tool's remaining chain.
                if app.tool.is_some() {
                    crate::tools::finish(app);
                }
            } else if app.session.active_sketch.is_some() {
                app.finish_sketch();
            }
        }
        _ => {}
    }
}
