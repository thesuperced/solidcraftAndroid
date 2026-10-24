# Pen-first Android validation

The six review milestones extend the existing Android test branch. The integrated
stack reuses the shared engine, command dialogs, selection storage, sketch tools,
snapping and gizmos. It does not change the accepted camera gesture calculation or
its per-frame coalescing.

## Milestone changes

| Milestone | Main files | Behavior / validation |
|---|---|---|
| 1: native router | `vendor/winit/src/platform/android_cad.rs`, Android platform implementation, `finger_gestures.rs`, `input-tests.rs` | Stable device/pointer IDs, independent capture, pen metadata/history, hover and cancellation. Seven router/camera tests pass; ARM64 build passes. |
| 2: pen selection | `tablet_input.rs`, `selection.rs`, `select_other.rs`, `viewport.rs` | Add/toggle by pen, empty-space clear, atomic body promotion, hold-to-overlap picker. Actual UI replay covers repeated taps and repeated body double-taps. |
| 3: held box | `viewport.rs` | Hold empty space then drag; shared enclosed/crossing rules; All/Bodies/Faces/Edges/Vertices/Sketch filters. UI replay changes Faces filter while the pen stays pressed. All retains the shared context default (bodies in 3D, sketch geometry in a sketch). |
| 4: semantic dock | Android shell and configuration adapter; `tool_modifiers.rs`, `prefs.rs`, browser/timeline | Finger-ID-specific holds and latches; direct semantic actions with physical keyboard precedence. Native UI actions use egui accessibility activation to preserve the pen cursor. Tests cover interleaving, release, cancel, and desktop selection chords. |
| 5: sketch / precise values | `keypad.rs`, `dialogs_move.rs`, shared tablet actions | Existing tool/gizmo/snap pipeline; pen tap edits dimension; keypad beside its field; captured key tap stays focused; moving off a key cancels it; transform Copy mode uses existing move parameters. Five keypad tests and strict lint pass. UI replay verifies sketch creation/undo and gizmo changes without camera movement. |
| 6: integration | `tests/tablet_input.rs`, native input diagnostics, Android instrumentation sources | Ten shared UI replay tests pass. A separate offscreen egui render of the real dock was generated and visually inspected. Native instrumentation APK compiles and signs successfully. |

## Reproduce

```sh
rustc --edition=2024 --test tools/android/input-tests.rs -o /tmp/solidcraft-input-tests
/tmp/solidcraft-input-tests
cargo test -p solidcraft-ui-egui --test tablet_input --locked -- --test-threads=1
cargo test --manifest-path tools/android/keypad-tests/Cargo.toml --locked
cargo clippy --manifest-path tools/android/keypad-tests/Cargo.toml --all-targets --locked -- -D warnings
SOLIDCRAFT_INPUT_PREVIEW=/tmp/tablet-controls.png cargo test -p solidcraft-ui-egui --test tablet_input render_tablet_controls_for_review -- --ignored
```

The image is a host rendering of the actual widgets, not a physical-device
screenshot. The Android motion replay harness is documented in
`tools/android/instrumentation/README.md`.

## Remaining verification

No tablet is attached to this cloud workspace. Physical S Pen behavior, OEM palm
filtering, simultaneous-event delivery, native instrumentation execution,
rotation/resume and save/reopen still need device acceptance. Samsung can suppress
finger events while the pen is in proximity; the app accepts delivered off-hand
control events but cannot recover contacts the OS never reports. Concurrent
off-hand UI click/focus actions are supported; general multi-touch scrolling or
dragging of arbitrary desktop widgets is not claimed.

`cargo xtask ci` passes formatting and then stops at pre-existing missing asset
attribution rows for Android launcher and vendored Winit sample images. This is
not an all-workspace CI pass. The Android build retains the existing font/bridge
unused-code warnings. Shared desktop parity is covered by selection-chord replay,
sketch and gizmo tests; the entire desktop regression suite has not been run.

## Tablet acceptance order

1. Install the new test APK with `adb install -r`, retaining the app's data.
2. Select face A, then B, then A: both, then B only. Double-tap A twice: its body
   remains selected. Tap empty space to clear; hold geometry to open Select Other.
3. Hold uncovered empty viewport space, then drag. Check enclosed/crossing
   direction rules and change filters with an off-hand finger while the pen stays
   down. The dock can switch sides.
4. Tap a dock action to latch/unlatch; hold it for a temporary action. Check
   No Snap, Midpoint, Free, Reselect and Copy only in their applicable tools.
5. Draw and undo a sketch; drag a gizmo and confirm the camera stays still. Tap
   a dimension, type a value, backspace/re-edit, confirm with ✓ and cancel with ✕.
6. Cancel a pen gesture, background/resume, rotate, and save/reopen a test design.
   Recheck the already accepted one-/two-finger camera gestures and physical keys.
