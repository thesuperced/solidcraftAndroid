# SolidCraft PenTouch experimental APK

Repository: https://github.com/thesuperced/solidcraftAndroid
Branch: `feat/android-pen-touch-keypad`
Commit: `a5681007839488288be5b5ae56d4b3d06d0d11c9`
PR: https://github.com/thesuperced/solidcraftAndroid/pull/1
Base: Hawaii-005/solidcraft `android-latest`, `ada94401efc3a589817b14bbf2606b4b8072340b`.
Base vendored winit tree: `df9c41652fb44b724293961ab13d25ec03da7cf7`.
Android template pin: `07ab398456a4356744c78a598599a78ba26f5df4`.
Additional dependency versions are pinned in the committed Cargo.lock files.

## APK

Filename: `solidcraft-pentouch-test-a5681007-arm64.apk` (~38 MiB).
SHA-256: `4fc4d6e41311ab4c8c98d711bd970a2dba4f09ed83e7a493817957da37ee0b03`.
For direct GitHub branch delivery, the original APK was repackaged with ZIP
compression for its native library, aligned again, and signed with the same debug
key. Android's default native-library extraction policy supports compressed libs.
The manifest, native library and classes.dex were compared byte-for-byte against
the original build and match. Signature and 16 KB alignment verification pass.
Package: `ai.storyteller.solidcraft.pentouch`.
Label: SolidCraft PenTouch Test.
Version: 0.18.3 (versionCode 16781827), arm64-v8a, minSdk 28, targetSdk 35.
Launcher: `ai.storyteller.solidcraft.SolidCraftActivity`.
Signing: locally generated RSA-2048 Android debug key; APK v3 signature verified.
Certificate SHA-256: `6b2d3043a7bd55a83b0300fcd708d829ef3d43f3d2e1eac30428b18756e76223`.
No key or signing password is committed or distributed.

Built from the committed source using Rust 1.99.1, JDK 21, cargo-apk2 1.5.0,
Android platform 35, build tools 35.0.0 and NDK 29.0.14206865:

```sh
export JAVA_HOME=/workspace/toolchain/jdk
export PATH="$JAVA_HOME/bin:/workspace/toolchain/cargo/bin:$PATH"
export CARGO_HOME=/workspace/toolchain/cargo
export RUSTUP_HOME=/workspace/toolchain/rustup
export ANDROID_HOME=/workspace/android-sdk
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export ANDROID_USER_HOME=/workspace/android-test-signing
export CARGO_TARGET_DIR=/workspace/solidcraftAndroid/target-android
export CARGO_PROFILE_DEV_DEBUG=0
cd /workspace/solidcraftAndroid/apps/solidcraft-android
cargo apk2 build --target aarch64-linux-android
```

## Passed

- Dependency-free actual-source input tests: `rustc --edition=2024 --test tools/android/input-tests.rs -o /tmp/solidcraft-input-tests && /tmp/solidcraft-input-tests` (3 tests).
- `cargo test --manifest-path tools/android/keypad-tests/Cargo.toml` (4 tests).
- `cargo check -p solidcraft-ui-egui`.
- `cargo clippy -p solidcraft-ui-egui --locked -- -D warnings`.
- `cargo clippy --manifest-path tools/android/keypad-tests/Cargo.toml --all-targets -- -D warnings`.
- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo xtask wasm` (all 12 checked crates).
- `cargo xtask licences --check`.
- Final `cargo apk2 build --target aarch64-linux-android`.
- `apksigner verify --verbose --print-certs`, manifest/ABI inspection using `aapt dump badging`, and `zipalign -c -P 16 4`.

## Failed or not verified

- `cargo xtask ci`: stops at inherited missing asset attribution for Android launcher images and vendored winit sample images. Formatting passes first.
- `cargo xtask layers`: inherited Android app registration and UI-to-MCP same-layer dependency violations.
- Full native workspace suite and strict Android Clippy were not completed. Android build emits existing unused-code warnings in font/host services.
- Physical tablet tests, emulator tests, screenshots, hardware keyboard behavior, S Pen picking/drawing regression, palm rejection, simultaneous pen/finger support, performance, and SAF save/reopen: NOT RUN.

## Implementation

Typed per-pointer routing precedes egui synthesis. Captured viewport fingers cannot select geometry; pen contacts cannot join finger gestures. Gestures calculate independent deltas and centroid/span ratios, handle cancellation and contact transitions, and use the shared camera command path. Sketch mode pans to preserve the sketch plane. A bounded transport preserves ownership even on queue overflow.

The five-column keypad includes numeric/ABC pages, sticky Shift/Ctrl, focus-safe next-frame text/key events, Clear, Undo/Redo, units, collapse/pin modes, resize, opacity and persisted numeric-key reordering. Both winit and Java automatic IME paths are suppressed. A test that actually clicks a keypad button verifies focus restoration and replacement of selected text.

Main changed files: `vendor/winit/src/platform/android_cad.rs`, `vendor/winit/src/platform/android.rs`, `vendor/winit/src/platform_impl/android/mod.rs`, `apps/solidcraft-android/src/lib.rs`, `src/keypad.rs`, `src/finger_gestures.rs`, Android Cargo.toml and Cargo.lock, `crates/ui-egui/src/navigation.rs`, `lib.rs`, `viewport.rs`, input/keypad test harnesses, and the template synchronization script. The initial branch commit also updates the Activity's focus policy. Four existing Android bridge files received formatter-only changes. No geometry-kernel implementation changes.

## Installation and tablet testing

Keep the original SolidCraft installed. Install this APK and launch **SolidCraft PenTouch Test**; it has independent files/preferences. Export a copy of a design through SAF if needed.

Test pen select/sketch/hover, one-finger orbit, two-finger pan/pinch, UI-started contact capture, simultaneous pen/finger input, resting palm and cancellation. Focus a dimension and enter `-12.5`, replace/backspace/Enter/Undo; try ABC names, sticky Shift multiselect and a hardware keyboard. Rotate, background/resume, then save/reopen a copied design and verify dimensions.

Full script and architecture: https://github.com/thesuperced/solidcraftAndroid/blob/feat/android-pen-touch-keypad/docs/android-pen-touch-keypad.md

This is a test build awaiting tablet acceptance. OS/OEM palm filtering can suppress simultaneous pen/finger streams; the software cannot establish that capability without hardware evidence.
