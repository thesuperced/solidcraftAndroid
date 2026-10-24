# Android pen, finger navigation and CAD keypad test build

Source: `thesuperced/solidcraftAndroid`, branch `feat/android-pen-touch-keypad`.
Base: `android-latest`, `ada94401efc3a589817b14bbf2606b4b8072340b`.
Base vendored winit tree: `df9c41652fb44b724293961ab13d25ec03da7cf7`.
Android bridge template pin: `07ab398456a4356744c78a598599a78ba26f5df4`.
The workspace version is 0.18.3. Dependencies are resolved in Cargo.lock;
winit and android-activity changes remain in the existing vendored copies.

## Input transport

The Android backend reads each pointer's tool, stable device/pointer identity,
position, pressure, timestamp and button flags before egui sees input.
Viewport fingers are captured on down, queued through `winit::platform::android::cad`,
and consumed before egui touch-to-pointer synthesis. Panel fingers continue into egui.
Starts/ends use the action pointer; move/cancel visits every affected pointer.
Pen and eraser contacts become pointer events and cannot join a finger pinch.
Hover remains a cursor move. Tool classification never uses pressure, hover, or ID bits.

The shell excludes floating panels and navigation controls and checks egui layers.
Ownership persists across boundaries. The finger-only state machine computes its own
deltas and centroid/span ratio. A second contact re-baselines immediately; after a
pinch, the remaining finger stays inert until all contacts lift. Extra contacts
are inert while more than two are down. Sketch mode uses one-finger pan to preserve
the sketch plane. Camera motions go through the shared `view.navigate` path;
Android generic-touch navigation is disabled to avoid double zoom.

OS cancellation clears gestures; focus loss/pause clears routing. FLAG_CANCELED
is recognized, including canceled pointer-up. Pen cancellation requests Escape and
releases the synthetic pointer outside the canvas before it can become a click.
There is no unconditional stylus-down finger suppression. OEM palm filtering and
simultaneous pen/finger delivery require hardware validation, especially before Android 13.

## Keypad

Left dock, five columns, numeric and ABC pages, sticky Shift/Ctrl, Clear,
Backspace/Delete, Enter/Tab/Escape, Undo/Redo and unit chips. Pin keeps it open;
unpinned mode follows text focus; Collapse leaves a small CAD launcher.
Resize the dock by its edge. Settings provide opacity and numeric-key reordering.
The saved layout is local to this app; typed text, focus and sticky modifiers are
never serialized. Modifiers remain enabled across tool changes until toggled off;
focus loss clears them. Hardware modifier state is combined with keypad modifiers.

Actions are queued for the next RawInput frame. Focus restoration occurs before
that frame's input processing: restoring it only during drawing is insufficient,
because a TextEdit later in the same frame can surrender focus after the button click.
Java focus auto-IME and winit's automatic show-soft-input are both suppressed.
The template synchronization script preserves the Java policy after regeneration.

## Reproduce the build

Use Rust stable 1.95+, a full JDK 21 (including javac), Android platform 35,
build-tools 35.0.0, NDK 29.0.14206865, and cargo-apk2 1.5.0:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk2 --version 1.5.0 --locked
sdkmanager 'platforms;android-35' 'build-tools;35.0.0' 'ndk;29.0.14206865' 'platform-tools'
export ANDROID_HOME=/path/to/android-sdk
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export JAVA_HOME=/path/to/jdk21
export PATH="$JAVA_HOME/bin:$PATH"
export ANDROID_USER_HOME=/path/to/ignored/local-android-settings
export CARGO_PROFILE_DEV_DEBUG=0
cd apps/solidcraft-android
cargo apk2 build --target aarch64-linux-android
```

Use the debug build's local test signing key. No signing key or release password
is committed. The distinct package `ai.storyteller.solidcraft.pentouch`, labelled
**SolidCraft PenTouch Test**, installs beside the original app. Its files and
preferences are separate. Export a copy of any design you want to test through SAF.

For a smaller downloadable APK, use `bash tools/android/package-test-apk.sh INPUT OUTPUT`
from the repository root, with ANDROID_HOME, APK_TEST_KEYSTORE and APK_SIGNING_PASSWORD
set locally. It preserves the required uncompressed/aligned resource table. Run
`ANDROID_HOME=/path/to/sdk bash tools/android/verify-test-apk.sh OUTPUT` before
publishing. Signature checks alone do not detect the compressed-resource-table
installation error on Android 11+. The earlier incorrectly repackaged download
has been superseded by `solidcraft-pentouch-test-installfix-arm64.apk`.

## Automated verification

```sh
rustc --edition=2024 --test tools/android/input-tests.rs -o /tmp/solidcraft-input-tests
/tmp/solidcraft-input-tests
cargo test --manifest-path tools/android/keypad-tests/Cargo.toml
cargo check -p solidcraft-ui-egui
cargo fmt --all -- --check
cargo xtask ci
```

The standalone tests compile the actual routing, gesture and keypad source files.
They exercise ownership, mixed source exclusion, slop, pinch/lift, cancellation,
hostile coordinates, numeric entry/backspace, actual keypad clicks replacing selected
text, modifier combination and persistence privacy. These are synthetic host tests.
They do not establish a physical tablet pass.

## Tablet acceptance script

1. Keep the working SolidCraft installed. Launch SolidCraft PenTouch Test and create a simple box.
2. Pen-tap faces/edges/vertices; draw and finish a sketch. Check hover and precision picking.
3. Drag one finger over the box: orbit without selecting. Pan with two fingers and pinch;
   lift one finger and confirm no jump or accidental selection. During sketching, check pan.
4. Tap and scroll toolbars and floating panels; start a finger there and drag into the
   model, then start in the model and drag over a panel. Ownership should stay at its start.
5. Hold the pen down and navigate with fingers. Rest a palm while drawing. In a debug
   build the bottom-right overlay shows recent input; collect `adb logcat` if delivery differs.
6. Focus a dimension, select its text, tap `-12.5`, backspace, replace it, Enter, Undo and
   re-edit. The OS keyboard must stay hidden. Use ABC for a filename or object name.
7. Toggle Shift, pen-select multiple faces/edges/vertices, toggle it off and select again.
   Attach a physical keyboard and check typing, Shift/Ctrl shortcuts and Tab focus changes.
8. Collapse, unpin, resize, reorder keys, adjust opacity and restart to check saved settings.
9. Interrupt a gesture by switching apps, resume and rotate the screen. Check no stuck input.
10. Save a copied design through SAF, reopen it and verify dimensions.

Physical-device tests, SAF round-trip, graphics performance, and OEM palm rejection
are NOT RUN in this environment. This is an experimental test build, not a verified release.

## Validation notes

The host UI check and targeted UI Clippy (`--locked -- -D warnings`) pass.
Keypad Clippy with all targets and warnings denied also passes. `cargo xtask wasm`
passes for all 12 checked crates; workspace `cargo fmt --all -- --check` passes.
`cargo xtask licences --check` passes (generated notices are deferred to the project's
normal Generated files workflow). The arm64 Android native and Java code compile;
the APK is debug-signed for tablet testing, with no physical-device pass claimed.
The three router/gesture tests and four keypad tests pass. The keypad suite includes
an actual pointer press/release on a keypad button, followed by queued text input.
Android bridge files also received formatter-only updates to satisfy workspace formatting.

The inherited Android fork does not pass all repository audits: `xtask assets`
reports missing attribution for the Android launcher images and vendored winit sample
images; `xtask layers` reports an unregistered Android app and the existing UI-to-MCP
same-layer dependency. These issues are outside the input implementation and remain
visible rather than weakening the audits. No geometry-kernel code is changed.
