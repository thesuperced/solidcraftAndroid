# Android control-panel test build

This revision implements the approved tablet feedback. It replaces the semantic
modifier dock with a permanent compact control column. The existing contextual
value keypad remains, with German umlauts added to ABC mode.

## Panel

Top to bottom: half-height docking arrow, CTRL, Cancel, TAB, Delete, QP, Redo,
Undo, Hide, Finish. CTRL is momentary: releasing or canceling its contact always
releases it, including short taps. It uses the same selection-toggle action as
physical Ctrl. Browser and timeline selections also toggle correctly.

Drag the arrow with the pen to move the column. Tap it to switch to the default
position on the other side and discard the custom position. Hide leaves a narrow
restore tab at the left edge. The panel position, side, visibility and seven QP
assignments are saved between launches; held input is never saved.

QP opens a second column to the right. Tap **Edit slots** to enter up to seven
keyboard shortcuts, for example `M`, `E`, or `Ctrl+Shift+K`, then tap **Done**.
Long-holding a slot also opens editing. Shortcuts use the existing keyboard and
command paths, including user key bindings and context-dependent availability.
An empty or invalid slot reports how to configure it. Slots are initially empty.

Cancel ends the active tool/dialog, clears selection and ignores the rest of any
active pen contact. Finish ends a drawing tool, accepts a dialog through its normal
Enter path, or finishes the sketch when no tool/dialog is active. TAB and Delete
use the existing keyboard paths, including text-field behavior when editing.
Undo/Redo use the existing engine commands. The former floating contextual
action buttons are removed.

## Selection and concurrent input

Pen selection is additive/toggling when no drawing tool or dialog owns the pen.
Tap **Cancel** after drawing to put down the current tool before testing selection.
Tap a sketch curve to select it; repeat after the double-tap interval to deselect.
Hold CTRL with a finger and tap multiple curves, then a selected curve to remove it.
Double-tap a sketch curve to select its connected chain. Double-tap a 3D face to
select its body; a sketch profile is not a solid body.

Selection filters reset to All when entering/leaving a sketch or changing tool
context, preventing a Faces/Bodies filter from hiding sketch selection. During a
held box gesture, the filter buttons retain independent finger captures; they do
not move or release the pen cursor. CTRL holds also survive tool-context changes.

For this first test build, delivered viewport finger contacts are allowed to move
the camera during pen contact, as approved. Finger contacts never select viewport
geometry. Panel controls have priority over camera navigation. The final requested
policy (block canvas camera movement during pen contact while keeping panel input)
is deferred until physical concurrency is confirmed. No Android/Samsung system
settings are changed. Samsung's delivery of real concurrent contacts still needs
confirmation on the tablet; injected contacts do not prove hardware delivery.

## Tablet acceptance order

1. Draw two connected sketch lines, tap Cancel, then test tap/toggle and CTRL-held
   add/remove. Test double-tap chain selection separately from slow repeat taps.
2. Select a 3D face, then double-tap it to promote to the body.
3. Hold empty space and drag for box selection. Keep the pen down and change each
   filter with a finger. Finish the gesture and inspect the selected entity types.
4. Hold CTRL before and after bringing the pen near the display; release it and
   verify its highlight and action both stop. Repeat while the pen touches down.
5. Try orbit/pan/pinch with the pen hovering, and with it touching, for this test.
6. Drag the panel arrow with the pen, tap it to dock on each side, Hide and restore.
7. Assign `M` to a QP slot, close editing, select a body and tap the slot. Verify
   Move opens; cancel it. Test QP on both dock sides without overlapping columns.
8. Draw geometry, Undo, Redo. Select geometry and Delete, then Undo.
9. While drawing, use Cancel with a finger. Lifting the pen must not add a point or
   select geometry. Finish a tool, then Finish the sketch in a separate press.
10. Open an operation with multiple numeric fields. Enter a value, use panel TAB
    to advance, then Finish to apply. Verify the model and Undo it.
11. In a text field, enter `äöü`; use Shift for `ÄÖÜ`. Verify Backspace and commit.
12. Restart the app and check panel placement and QP assignments were retained.

The Windows updater stops the running app, checks the APK SHA-256, installs with
`adb install -r` and launches it. Do not uninstall: the package and signing key
are retained so designs and preferences can survive an update.
