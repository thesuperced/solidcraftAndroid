# Android pen-first input

## Architecture and sequence

`MotionEvent → vendor/winit/.../android_cad.rs::CadInputRouter → Android Shell → egui viewport / semantic actions → SolveApp::run → Session`

The native bridge normalizes device + pointer ID (never pointer index), tool type,
phase, position, pressure, tilt, orientation, time, buttons and historical samples.
Ownership is assigned at down and retained until up/cancel: explicit control, UI,
pen modeling, camera, or rejected palm. UI hit testing precedes viewport routing.
Pen modeling continues through the existing viewport; canvas/gizmo widgets have
priority over its background response. Finger camera samples keep the accepted
per-frame coalescing path through `navigation::tablet_motion`.

Reuse: `viewport::candidate`, `select`, `dialog_pick`, `box_select`;
`selection::BoxSel` and `same_item`; `select_other::track_hold` / `choose`;
`tool_modifiers::resolve`; `tools::on_click` / `on_hover`; existing canvas,
gizmo, sketch inference/drag and dimension editors. No modeling-engine fork.

Implementation milestones:

1. Normalize native metadata, capture ownership and deterministic router tests.
2. Pen additive/toggle selection, atomic body promotion and overlap picking.
3. Hold-to-box selection and direction-aware shared selection with off-hand filters.
4. Semantic modifier dock: latches and pointer-ID-specific momentary holds;
   physical keyboard bindings remain active. Selection toggle wins over add;
   suppression wins over snapping. Tool-scoped latches reset on tool transitions.
5. Reuse sketch/gizmo/snapping/value paths; single pen tap edits dimensions;
   contextual commands and numeric entry use shared commands.
6. Cancellation/palm/interleaving replay, Android build, package verification,
   and device acceptance checklist. Hardware results must be reported separately.

Platform gesture thresholds come from Android ViewConfiguration with bounded
fallbacks when unavailable. OEM suppression of finger events during pen proximity
cannot be reversed by application code; explicit controls accept all events the OS
delivers. Pressure never determines CAD dimensions.
