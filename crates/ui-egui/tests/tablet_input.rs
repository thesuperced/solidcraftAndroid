use egui::pos2;
use serde_json::json;
use solidcraft_engine::{Sel, geom::Vec3};
use solidcraft_ui_egui::{
    scenario::Harness,
    selection::{BoxSel, promote_body},
    tablet_input::{self as tablet, Dock, Policy, Transaction},
};

struct Tablet;
impl Tablet {
    fn pen() -> Self {
        tablet::set(Policy { enabled: true, pen: true, ..Default::default() });
        Self
    }
}
impl Drop for Tablet {
    fn drop(&mut self) {
        tablet::set(Policy::default());
    }
}

#[test]
fn pen_taps_add_toggle_empty_clear_and_double_tap_promotes_body() {
    let mut h = Harness::new();
    h.app.ui.show_origin = false;
    h.app.run("solid.box", json!({"length":40,"width":40,"height":40})).unwrap();
    h.app.cam.distance = 180.0;
    h.settle();
    let _tablet = Tablet::pen();
    let a = json!({"world":[20,20,40]});
    let b = json!({"world":[40,20,20]});
    h.step(&json!({"click": a})).unwrap();
    assert_eq!(h.app.session.selection.len(), 1);
    h.frames(40);
    h.step(&json!({"click": a})).unwrap();
    assert!(h.app.session.selection.is_empty(), "a repeated pen tap toggles instead of cycling hidden geometry");
    h.frames(40);
    h.step(&json!({"click": a})).unwrap();
    h.frames(40);
    h.step(&json!({"click": b})).unwrap();
    assert_eq!(h.app.session.selection.len(), 2);
    h.frames(40);
    h.step(&json!({"click": a})).unwrap();
    assert_eq!(h.app.session.selection.len(), 1);
    h.frames(40);
    h.step(&json!({"click": a, "double":true})).unwrap();
    assert!(matches!(h.app.session.selection.as_slice(), [Sel::Body { .. }]));
    h.frames(40);
    h.step(&json!({"click": a, "double":true})).unwrap();
    assert!(matches!(h.app.session.selection.as_slice(), [Sel::Body { .. }]));
    let r = h.app.viewport.rect.unwrap();
    h.frames(40);
    h.step(&json!({"click": [r.right()-30.0, r.bottom()-110.0]})).unwrap();
    assert!(h.app.session.selection.is_empty());
}

#[test]
fn body_promotion_reconciles_first_tap_and_retains_other_bodies() {
    let face = Sel::Face { body: "A".into(), index: 1, point: Vec3::ZERO };
    let other = Sel::Body { name: "B".into() };
    assert_eq!(promote_body(&[face, other.clone()], "A"), vec![other, Sel::Body { name: "A".into() }]);
}

#[test]
fn pen_sketch_uses_shared_tool_preview_creation_and_undo() {
    let mut h = Harness::new();
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.cam.distance = 90.0;
    h.settle();
    let _tablet = Tablet::pen();
    let sketch = h.app.session.active_sketch.unwrap();
    let count = h.app.session.doc.sketch(sketch).unwrap().curves.len();
    h.app.start("sketch.line");
    h.step(&json!({"click":{"sketch":[5,5]}})).unwrap();
    h.step(&json!({"move":{"sketch":[25,5]}})).unwrap();
    assert!(h.app.tool.is_some(), "a pen click remains in the active sketch tool");
    h.step(&json!({"click":{"sketch":[25,5]}})).unwrap();
    assert!(h.app.session.doc.sketch(sketch).unwrap().curves.len() > count);
    h.app.run("edit.undo", json!({})).unwrap();
    assert_eq!(h.app.session.doc.sketch(sketch).unwrap().curves.len(), count);
}

#[test]
fn pen_sketch_curve_toggle_ctrl_selection_and_double_tap_chain() {
    let mut h = Harness::new();
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.cam.distance = 90.0;
    h.app.ui.show_origin = false;
    h.settle();
    let _tablet = Tablet::pen();
    h.app.start("sketch.line");
    for point in [[5, 5], [25, 5], [25, 25]] {
        h.step(&json!({"click":{"sketch":point}})).unwrap();
    }
    tablet::panel_action(&mut h.app, 21);
    h.frames(40);
    let a = json!({"sketch":[15,5]});
    let b = json!({"sketch":[25,15]});
    h.step(&json!({"click":a})).unwrap();
    assert!(matches!(h.app.session.selection.as_slice(), [Sel::SketchCurve { .. }]), "{:?}", h.app.session.selection);
    h.frames(40);
    h.step(&json!({"click":a})).unwrap();
    assert!(h.app.session.selection.is_empty());
    let mut p = tablet::policy();
    p.actions[tablet::ADD] = true;
    tablet::set(p);
    h.frames(40);
    h.step(&json!({"click":a})).unwrap();
    h.frames(40);
    h.step(&json!({"click":b})).unwrap();
    assert_eq!(h.app.session.selection.len(), 2);
    h.frames(40);
    h.step(&json!({"click":a})).unwrap();
    assert_eq!(h.app.session.selection.len(), 1, "held CTRL must allow deselection");
    p.actions = [false; 7];
    tablet::set(p);
    h.frames(40);
    h.step(&json!({"click":a,"double":true})).unwrap();
    assert_eq!(h.app.session.selection.len(), 2, "double tap selects the connected sketch chain");
}

#[test]
fn panel_dispatch_undo_redo_finish_shortcuts_and_context_reset() {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":20,"width":20,"height":20})).unwrap();
    tablet::panel_action(&mut h.app, 27);
    assert!(h.app.session.world_state().bodies.is_empty());
    tablet::panel_action(&mut h.app, 26);
    assert_eq!(h.app.session.world_state().bodies.len(), 1);
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.start("sketch.line");
    tablet::panel_action(&mut h.app, 29);
    assert!(h.app.tool.is_none());
    tablet::panel_action(&mut h.app, 29);
    assert!(h.app.session.active_sketch.is_none());
    let mut dock = Dock::default();
    dock.filter = tablet::Filter::Faces;
    dock.context("sketch".into());
    assert_eq!(dock.filter, tablet::Filter::All);
    assert!(tablet::key_events("Ctrl+Shift+M").is_some());
    assert!(tablet::key_events("Tab").is_some());
    assert!(tablet::key_events("Delete").is_some());
    assert!(tablet::key_events("not a shortcut").is_none());
}

#[test]
fn panel_drag_dock_hide_restore_and_shortcut_preferences() {
    let ctx = egui::Context::default();
    let mut dock = Dock::default();
    let draw = |dock: &mut Dock| {
        let mut regions = Vec::new();
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), ..Default::default() },
            |ui| {
                regions = dock.show(ui.ctx(), false, false, false);
            },
        );
        output.textures_delta.clear();
        regions
    };
    draw(&mut dock);
    let regions = draw(&mut dock);
    let arrow = regions.iter().find(|(id, _)| *id == 20).unwrap().1.center();
    let at = [arrow.x, arrow.y];
    dock.contact(1, 7, 20, true, false, at, 0.0, 0.5, 8.0);
    let moved = [at[0] + 150.0, at[1] + 40.0];
    dock.move_contact(1, 7, moved, true, 8.0);
    dock.contact(1, 7, 20, false, false, moved, 0.3, 0.5, 8.0);
    let regions = draw(&mut dock);
    let next = regions.iter().find(|(id, _)| *id == 20).unwrap().1.center();
    assert!((next.x - arrow.x - 150.0).abs() < 1.0);
    assert!(!dock.right, "dragging must not also switch dock sides");
    for down in [true, false] {
        dock.contact(1, 7, 20, down, false, [next.x, next.y], 1.0, 0.5, 8.0);
    }
    assert!(dock.right);
    for down in [true, false] {
        dock.contact(1, 7, 28, down, false, at, 2.0, 0.5, 8.0);
    }
    assert_eq!(draw(&mut dock).iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![30]);
    for down in [true, false] {
        dock.contact(1, 7, 30, down, false, at, 3.0, 0.5, 8.0);
    }
    assert!(draw(&mut dock).iter().any(|(id, _)| *id == 29));
    dock.shortcuts[0] = "M".into();
    let restored: Dock = serde_json::from_str(&serde_json::to_string(&dock).unwrap()).unwrap();
    assert_eq!(restored.shortcuts[0], "M");
    assert!(restored.right);
    assert_eq!(restored.actions(), [false; 7]);
}

#[test]
fn pen_gizmo_drag_changes_dialog_value_without_moving_camera() {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":20,"width":20,"height":20})).unwrap();
    let body = h.app.session.world_state().bodies[0].name.clone();
    h.app.run("select.set", json!({"items":[{"type":"body","name":body}]})).unwrap();
    h.app.start("solid.move");
    h.settle();
    let _tablet = Tablet::pen();
    let before = h.app.cam.clone();
    let at = solidcraft_ui_egui::scenario::handle_at("move_x").expect("shared move gizmo X handle");
    h.step(&json!({"drag":[[at.x,at.y],[at.x+50.0,at.y-10.0]],"steps":8})).unwrap();
    assert_eq!(h.app.cam, before);
    let dialog = h.app.dialog.as_ref().unwrap();
    if let solidcraft_ui_egui::dialogs::Kind::Move { x, .. } = &dialog.kind {
        assert_ne!(x, "0 mm");
    } else {
        panic!("move dialog disappeared");
    }
}

#[test]
fn box_direction_rules_use_shared_geometry() {
    let enclosed = BoxSel::from_drag(pos2(0.0, 0.0), pos2(100.0, 100.0));
    let crossing = BoxSel::from_drag(pos2(100.0, 0.0), pos2(0.0, 100.0));
    let segment = [pos2(50.0, 50.0), pos2(150.0, 50.0)];
    assert!(!enclosed.takes(&segment));
    assert!(crossing.takes(&segment));
}

#[test]
fn held_pen_box_accepts_filter_change_without_releasing_pen() {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":20,"width":20,"height":20})).unwrap();
    h.app.ui.show_origin = false;
    h.app.cam.distance = 130.0;
    h.settle();
    let _tablet = Tablet::pen();
    let r = h.app.viewport.rect.unwrap();
    let mut start = r.left_top() + egui::vec2(20.0, 240.0);
    for cover in solidcraft_ui_egui::navigation::tablet_exclusions(&h.app) {
        if cover.contains(start) {
            start.x = cover.right() + 20.0;
        }
    }
    let end = r.right_bottom() - egui::vec2(20.0, 80.0);
    h.app.synthetic.push(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    h.frames(45);
    h.app.synthetic.push(egui::Event::PointerMoved(end));
    h.frames(3);
    assert!(h.app.viewport.boxsel.is_some(), "holding empty space then dragging arms box selection");
    let mut p = tablet::policy();
    p.filter = tablet::Filter::Faces;
    tablet::set(p);
    h.frames(2);
    assert!(h.ctx.input(|i| i.pointer.primary_down()));
    h.app.synthetic.push(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.frames(4);
    assert!(!h.app.session.selection.is_empty());
    assert!(h.app.session.selection.iter().all(|s| matches!(s, Sel::Face { .. })));
}

#[test]
fn off_hand_ctrl_holds_are_independent_and_release_never_latches() {
    let mut d = Dock::default();
    d.contact(1, 55, 0, true, false, [0.0; 2], 1.0, 0.5, 8.0);
    d.contact(1, 8, 0, true, false, [0.0; 2], 1.1, 0.5, 8.0);
    assert!(d.actions()[tablet::ADD]);
    d.contact(1, 55, 0, false, true, [0.0; 2], 1.15, 0.5, 8.0);
    assert!(d.actions()[tablet::ADD]);
    d.contact(1, 8, 0, false, false, [0.0; 2], 1.2, 0.5, 8.0);
    assert!(!d.actions()[tablet::ADD], "short Ctrl taps must not leave Ctrl stuck");
    d.reset();
    assert_eq!(d.actions(), [false; 7]);
}

#[test]
fn canceled_pen_transaction_restores_model_and_undo_redo() {
    let mut h = Harness::new();
    let before = h.app.session.doc.to_json();
    let undo = h.app.session.undo.len();
    let redo = h.app.session.redo.len();
    let mut transaction = Transaction::default();
    transaction.begin(&mut h.app);
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10})).unwrap();
    transaction.cancel(&mut h.app);
    assert_eq!(h.app.session.doc.to_json(), before);
    assert_eq!((h.app.session.undo.len(), h.app.session.redo.len()), (undo, redo));
    assert!(h.app.viewport.point_drag.is_none());
    assert!(h.app.viewport.boxsel.is_none());
}

#[test]
fn desktop_modifiers_remain_independent_of_tablet_policy() {
    use solidcraft_ui_egui::tool_modifiers::{Held, selection_click};
    assert_eq!(selection_click(Held::default(), false), (false, false));
    assert_eq!(selection_click(Held { shift: true, ..Default::default() }, true), (true, false));
    assert_eq!(selection_click(Held { primary: true, ..Default::default() }, true), (true, true));
    let _tablet = Tablet::pen();
    let mut p = tablet::policy();
    p.actions[tablet::ADD] = true;
    tablet::set(p);
    assert_eq!(selection_click(Held::default(), true), (true, true));
    assert_eq!(selection_click(Held { primary: true, ..Default::default() }, true), (true, true));
}

#[test]
fn off_hand_click_does_not_move_or_release_the_pen_cursor() {
    let ctx = egui::Context::default();
    tablet::install(&ctx);
    let pen = egui::pos2(400.0, 300.0);
    let mut pressed = false;
    let mut target = None;
    let draw = |ctx: &egui::Context, events, pressed: &mut bool, target: &mut Option<egui::accesskit::NodeId>| {
        let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            let response = ui.button("Off hand");
            *target = Some(response.id.accesskit_id());
            *pressed |= response.clicked();
        });
        out.textures_delta.clear();
    };
    draw(
        &ctx,
        vec![
            egui::Event::PointerMoved(pen),
            egui::Event::PointerButton { pos: pen, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
        ],
        &mut pressed,
        &mut target,
    );
    let hit = tablet::off_hand_target(&ctx, [10.0, 10.0]).expect("last output's button target");
    assert_eq!(hit.0, target.unwrap());
    draw(&ctx, vec![tablet::off_hand_action(hit.0, hit.1)], &mut pressed, &mut target);
    assert!(pressed);
    assert!(ctx.input(|i| i.pointer.primary_down()));
    assert_eq!(ctx.input(|i| i.pointer.interact_pos()), Some(pen));
}

/// Render the actual tablet widgets with egui's GPU renderer, without a window.
#[test]
#[ignore = "explicit visual review: set SOLIDCRAFT_INPUT_PREVIEW to a PNG path"]
fn render_tablet_controls_for_review() {
    use egui_wgpu::wgpu;
    let path = std::env::var("SOLIDCRAFT_INPUT_PREVIEW").expect("preview output path");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("offscreen GPU adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = egui_wgpu::Renderer::new(&device, format, Default::default());
    let extent = wgpu::Extent3d { width: 1600, height: 1024, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tablet review"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut app = solidcraft_ui_egui::SolveApp::new(Default::default(), Default::default());
    let ctx = egui::Context::default();
    solidcraft_ui_egui::fonts::setup_fonts(&ctx);
    let mut dock = Dock::default();
    dock.shortcuts[0] = "M".into();
    for down in [true, false] {
        dock.contact(1, 7, 25, down, false, [0.0; 2], 1.0, 0.5, 8.0);
    }
    for frame in 0..24 {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1024.0))),
            focused: true,
            time: Some(frame as f64 / 60.0),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
            dock.show(ui.ctx(), false, false, true);
        });
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&device, &queue, *id, delta);
            }
        }
        output.textures_delta.clear();
        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [1600, 1024], pixels_per_point: output.pixels_per_point };
        let mut encoder = device.create_command_encoder(&Default::default());
        let extra = renderer.update_buffers(&device, &queue, &mut encoder, &meshes, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            renderer.render(&mut pass, &meshes, &screen);
        }
        queue.submit(extra.into_iter().chain([encoder.finish()]));
    }
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 1600 * 1024 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(6400), rows_per_image: Some(1024) },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let slice = read.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = slice.get_mapped_range().unwrap();
    image::save_buffer(&path, &bytes, 1600, 1024, image::ColorType::Rgba8).unwrap();
}
