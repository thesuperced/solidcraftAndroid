# Tablet feedback: gestures, panel and sketch corners

This revision implements the next round of tablet feedback.

## Changes to test

- Three-finger **tap** invokes the configured middle-mouse double-click action
  (Home, Fit or Off in Navigation preferences). It does not require two taps.
  Three-finger drags, canceled contacts, holds longer than 500 ms and four-finger
  gestures do not trigger it. Use fingers on the canvas, not the control panels.
- The main column is now: split docking/hide row, TAB, Cancel, Finish, Delete, QP,
  Redo, Undo. CTRL and the old bottom Hide button are removed. In the split row,
  the outer half hides the panel; the arrow half switches dock sides and retains
  pen-drag repositioning. A narrow tab at the left restores the hidden panel.
- QP has nine shortcut slots. Seven-slot preferences from earlier APKs migrate
  without losing assignments; the two new slots start empty. Use **Edit slots**
  to assign shortcuts and **Done** to close editing.
- The contextual keypad opens at the lower right, independently of which field
  is focused. Drag **Move keypad** with the pen to reposition it. The custom
  position is saved and remains the same when changing fields. The numeric
  layout no longer includes deg, Undo, Redo or Delete. **X** retains Escape/cancel;
  **OK** commits. ABC retains German umlauts and Shift.
- Selected sketch curves use a thicker orange highlight from the existing theme
  palette, distinct from their normal constraint-state color and hover feedback.
- Experimental tablet TAB while drawing a line enters **Length** from the canvas;
  further presses cycle **Angle → Length**. Place the line's first point and move
  the pen to preview its endpoint before pressing TAB. Existing typed-value locks
  and Enter-to-place behavior remain in use.

## Multiple sketch corners

Previously Fillet accepted multiple point corners but only one selected pair of
curves; Chamfer accepted one corner. The UI now resolves shared eligible corners
from three or more selected lines/curves. Explicit corner points may be mixed in.
The engine applies all resolved references in one atomic command and one undo
step, including Equal Distance, Two Distance, and Distance/Angle Chamfer variants.
One failed corner rejects the batch; it must not leave earlier cuts behind.

Test on a fresh sketch rectangle:

1. Finish/cancel the drawing tool, then select all four sides with the pen.
2. Open **Sketch Fillet**, set a small radius, and Finish/apply. Check all four
   corners are rounded. Undo once: the complete original rectangle should return.
3. Repeat with each Sketch Chamfer type. Use small distances so cuts do not consume
   the whole side. Test Flip for the asymmetric variants.
4. Select three connected sides: only the two shared corners should change. A
   corner between a selected and unselected side is intentionally excluded.
5. Try a size too large to fit. The operation should report an error without
   leaving a partial result; reduce it and retry.

Corners must be shared editable endpoints. Collinear joins, linked geometry and
ambiguous junctions do not become invented corners. These are **sketch** tools;
the existing solid-edge Fillet/Chamfer tools keep their existing behavior.

## Device acceptance

Verify the panel on both dock sides, hide/restore and pen repositioning; all nine
QP assignments and their persistence; keypad position across fields and restart;
three-finger tap versus pan/zoom; selection color; and TAB value entry followed
by line placement. Physical tablet input, rotation/resume and installation require
device verification. The cloud workspace is not attached to the PC's USB device.

The Windows updater retains the application package and signing certificate,
verifies the APK checksum, stops the running app, installs with `adb install -r`
and launches it. Do not uninstall to update.
