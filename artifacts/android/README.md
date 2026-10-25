# Android sketch-corners and tablet-controls APK

Download [SolidCraft PenTouch Test — latest APK](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-sketch-corners-arm64.apk).

Arm64 Android 9 or newer, approximately 38 MiB.
Source: `008e46c7ac55d93d093365b6a14c1a951af73190`.
SHA-256: `6cc1f3cb9b1fd0ed7f39c587760024f99166440beb4e180f2d2d2a7be37c0ba2`.

This revision adds three-finger tap for the configured middle-button double-click
action, nine QP slots with migration of existing assignments, a split docking/hide
row, and the requested TAB / Cancel / Finish / Delete / QP / Redo / Undo order.
CTRL and the separate bottom Hide button are removed.

The keypad opens at the lower right and can be moved using its pen drag handle.
The numeric deg / Undo / Redo / Delete keys are removed; X cancels, OK commits.
Selected sketch curves have a thicker orange highlight. Experimental tablet TAB
enters Length while drawing a line and cycles Length / Angle.

Sketch Fillet and all three Sketch Chamfer variants accept multiple shared corners
along selected lines, with one atomic command and one undo step. A failed batch
does not leave partial geometry.
[Detailed behavior and device test steps](../../docs/android-sketch-corners-testing.md).

Use the [Windows updater](../../tools/android/update-tablet.ps1). It verifies the
download, stops the running app, installs with `-r` and launches it. The package
and signing certificate are unchanged; no uninstall is needed.

Passed: 8 native routing/gesture tests, 15 shared interaction tests (including all
four corner variants, failure rollback, single-step undo and TAB entry), 6 keypad
tests, strict keypad Clippy, workspace formatting, all 12 WebAssembly checks,
ARM64 build and Android packaging/signature checks. The actual nine-slot panel
was rendered offscreen and inspected. Full CI still encounters inherited missing
asset-attribution rows.

Physical S Pen/finger input and installation remain for tablet verification.
Earlier APKs are retained as recovery options.
