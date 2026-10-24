//! Tablet policy and semantic controls. Native contacts never become keyboard modifiers.
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
/// Context-valid modeling actions start the same dialogs as the desktop toolbar.
pub fn context_actions(app: &mut crate::SolveApp, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let Some(rect) = app.viewport.rect else { return };
    let commands: Vec<_> = if app.tool.is_none() && app.dialog.is_none() {
        use solidcraft_engine::Sel;
        let selected = &app.session.selection;
        [
            ("solid.extrude", "Extrude", selected.iter().any(|s| matches!(s, Sel::Profile { .. }))),
            ("solid.fillet", "Fillet", selected.iter().any(|s| matches!(s, Sel::Edge { .. }))),
            ("solid.move", "Move / Rotate", selected.iter().any(|s| matches!(s, Sel::Body { .. }))),
            ("solid.offset_face", "Offset Face", selected.iter().any(|s| matches!(s, Sel::Face { .. }))),
            ("sketch.create", "Sketch", selected.iter().any(|s| matches!(s, Sel::Face { .. } | Sel::Plane { .. }))),
        ]
        .into_iter()
        .filter(|(id, _, valid)| *valid && solidcraft_engine::find_command(id).is_some_and(|c| c.info(&app.session).enabled))
        .collect()
    } else {
        Vec::new()
    };
    let at = crate::viewport::selection_bounds(app)
        .and_then(|bounds| crate::viewport::projection(app, rect).to_screen(bounds.center()))
        .unwrap_or(rect.center());
    let position = egui::pos2(
        (at.x + 24.0).clamp(rect.left(), (rect.right() - 340.0).max(rect.left())),
        (at.y + 48.0).clamp(rect.top(), (rect.bottom() - 70.0).max(rect.top())),
    );
    if !commands.is_empty() {
        egui::Area::new(egui::Id::new("tablet_context_actions")).fixed_pos(position).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (command, label, _) in commands {
                        if ui.add_sized([94.0, 44.0], egui::Button::new(label)).clicked() {
                            app.start(command);
                        }
                    }
                });
            });
        });
    }
    egui::Area::new(egui::Id::new("tablet_tool_actions"))
        .fixed_pos(egui::pos2(rect.left() + 120.0, rect.bottom() - 118.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_sized([72.0, 44.0], egui::Button::new("Undo")).clicked() {
                        let _ = app.run("edit.undo", serde_json::json!({}));
                    }
                    if ui.add_sized([72.0, 44.0], egui::Button::new("Redo")).clicked() {
                        let _ = app.run("edit.redo", serde_json::json!({}));
                    }
                    if app.tool.is_some() {
                        if ui.add_sized([72.0, 44.0], egui::Button::new("Finish")).clicked() {
                            crate::tools::finish(app);
                        }
                        if ui.add_sized([72.0, 44.0], egui::Button::new("Cancel")).clicked() {
                            cancel(app);
                        }
                    }
                });
            });
        });
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// State keyed by device + pointer ID, independent of egui's single mouse cursor.
#[derive(Default)]
pub struct Dock {
    latched: [bool; 7],
    held: BTreeMap<(i32, i32), (u8, f64, [f32; 2])>,
    pub filter: Filter,
    pub right: bool,
    context: String,
}
impl Dock {
    pub fn reset(&mut self) {
        self.held.clear();
        self.latched = [false; 7];
    }
    pub fn context(&mut self, key: String) {
        if self.context != key {
            self.reset();
            self.context = key;
        }
    }
    pub fn actions(&self) -> [bool; 7] {
        let mut active = self.latched;
        for (id, _, _) in self.held.values() {
            if let Some(on) = active.get_mut(*id as usize) {
                *on = true;
            }
        }
        active
    }
    /// Down activates immediately. A short stationary release toggles the latch; a hold or
    /// canceled contact only releases its momentary action. Multiple fingers combine with OR.
    pub fn contact(&mut self, device: i32, pointer: i32, control: u8, down: bool, cancel: bool, at: [f32; 2], now: f64, hold: f64, slop: f32) {
        let key = (device, pointer);
        if down {
            self.held.entry(key).or_insert((control, now, at));
        } else if let Some((id, start, origin)) = self.held.remove(&key) {
            if cancel {
                return;
            }
            let tap = now >= start && now - start < hold && (at[0] - origin[0]).hypot(at[1] - origin[1]) <= slop;
            if tap {
                if let Some(latch) = self.latched.get_mut(id as usize) {
                    *latch = !*latch;
                } else if id == 20 {
                    self.right = !self.right;
                } else if let Some(filter) = id.checked_sub(10).and_then(|i| Filter::ALL.get(i as usize)) {
                    self.filter = *filter;
                }
            }
        }
    }
    pub fn show(&self, ctx: &egui::Context, sketch: bool, feature: bool, selecting_box: bool) -> Vec<(u8, egui::Rect)> {
        let mut regions = Vec::new();
        let active = self.actions();
        egui::Area::new(egui::Id::new("tablet_semantic_dock"))
            .anchor(if self.right { egui::Align2::RIGHT_CENTER } else { egui::Align2::LEFT_CENTER }, [if self.right { -12.0 } else { 12.0 }, 40.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (id, label, visible) in [
                        (0, "Add", true),
                        (1, "Range", true),
                        (2, "No Snap", sketch),
                        (3, "Midpoint", sketch),
                        (4, "Free", feature || sketch),
                        (5, "Reselect", feature),
                        (6, "Copy", feature),
                    ] {
                        if visible {
                            let r = ui.add_sized([82.0, 44.0], egui::Button::new(label).selected(active.get(id as usize).copied().unwrap_or(false)));
                            regions.push((id, r.rect));
                        }
                    }
                    let r = ui.add_sized([82.0, 44.0], egui::Button::new(if self.right { "Dock left" } else { "Dock right" }));
                    regions.push((20, r.rect));
                    if selecting_box {
                        ui.separator();
                        for (i, filter) in Filter::ALL.iter().enumerate() {
                            let r = ui.add_sized([82.0, 44.0], egui::Button::new(filter.label()).selected(self.filter == *filter));
                            regions.push((10 + i as u8, r.rect));
                        }
                    }
                });
            });
        regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interleaved_holds_latches_and_cancel_do_not_share_pointer_state() {
        let mut d = Dock::default();
        let at = [0.0, 0.0];
        d.contact(1, 91, 0, true, false, at, 0.0, 0.5, 8.0);
        d.contact(1, 3, 0, true, false, at, 0.1, 0.5, 8.0);
        assert!(d.actions()[ADD]);
        d.contact(1, 91, 0, false, true, at, 0.7, 0.5, 8.0);
        assert!(d.actions()[ADD]);
        d.contact(1, 3, 0, false, false, at, 0.8, 0.5, 8.0);
        assert!(!d.actions()[ADD]);
        d.contact(1, 91, 2, true, false, at, 1.0, 0.5, 8.0);
        d.contact(1, 91, 2, false, false, at, 1.1, 0.5, 8.0);
        assert!(d.actions()[NO_SNAP]);
        d.context("sketch.line".into());
        assert_eq!(d.actions(), [false; 7]);
    }
}
