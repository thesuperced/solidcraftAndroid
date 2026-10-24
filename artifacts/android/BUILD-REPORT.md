# Latest: pen-first Android test APK

Source commit: `bf64ff30d924cae8fb30db92b1a1d1fb6340fbef` (six implementation milestones).
App: `solidcraft-pen-first-arm64.apk` (~38 MiB).
SHA-256: `837f5965173128de3d47a0e50349771e105b9f0692127966ea3c7d1c81e7bfdd`.
Package: `ai.storyteller.solidcraft.pentouch`; Android 9+; arm64-v8a;
version 0.18.3; same test signing certificate as the installed previous build.

Passed: seven native router/camera tests; ten shared UI event replay tests; five
keypad tests; strict keypad Clippy; workspace formatting; ARM64 build; Android
resource storage, alignment, signature, ABI and launcher checks. The actual dock
was rendered offscreen through egui/WGPU and inspected. This render is not a
physical-device screenshot.
All 12 WebAssembly checks also pass.

`solidcraft-input-instrumentation.apk` is an optional, compiled and signed native
MotionEvent replay harness. It has not been executed on a device in this workspace.
Physical tablet acceptance remains open. `cargo xtask ci` stops at inherited
missing asset attributions. Full details, source changes per milestone and the
remaining acceptance cases: [pen-first validation](../../docs/android-pen-first-validation.md).

The build environment and packaging procedure below still apply. Previous
artifact checksums and implementation notes below are historical records.

# Previous contextual-keypad build

Repository: https://github.com/thesuperced/solidcraftAndroid
Branch: `feat/android-pen-touch-keypad`
Source commit: `90977ce15b862a046bf6d67f9058ff6753312e46`
PR: https://github.com/thesuperced/solidcraftAndroid/pull/1
Base: Hawaii-005/solidcraft `android-latest`, `ada94401efc3a589817b14bbf2606b4b8072340b`.
Base vendored winit tree: `df9c41652fb44b724293961ab13d25ec03da7cf7`.
Android template pin: `07ab398456a4356744c78a598599a78ba26f5df4`.
Additional dependency versions are pinned in the committed Cargo.lock files.

## APK

Filename: `solidcraft-context-keypad-arm64.apk` (~38 MiB).
SHA-256: `42b52bbdefe06ee0d27e7536902ccf8b3a74fb791507d018bbe305890cf91d3d`.
For direct GitHub branch delivery, the original APK was repackaged with ZIP
compression for its native library, aligned again, and signed with the same debug
key. Android's default native-library extraction policy supports compressed libs.
The first compressed download incorrectly compressed `resources.arsc` as well.
Android 11+ rejects that for target SDK 30+; signature verification alone does
not catch this. The replacement stores `resources.arsc` uncompressed at offset
39459996 (divisible by 4). The packaging regression check rejects the old APK
and accepts the replacement. No signing key, package ID or application code changed.
The manifest, native library and classes.dex were compared byte-for-byte against
the original build and match. Signature and 16 KB alignment verification pass.

Repackaging is now reproducible with `tools/android/package-test-apk.sh INPUT OUTPUT`
(set ANDROID_HOME, APK_TEST_KEYSTORE and APK_SIGNING_PASSWORD locally). It compresses
the native library, explicitly stores resources.arsc, aligns, signs, and runs
`tools/android/verify-test-apk.sh`. The latter verifies the resource-table storage
method before signature and alignment checks. Both scripts pass `bash -n`.

Galaxy Tab S7+ testing accepted one-finger orbit, coalesced two-finger pan/pinch,
S Pen hover and geometry selection, finger navigation without geometry selection,
and palm rejection. Testing then exposed that the permanent keypad's pointer event
closed the inline value editor before its queued key arrived. This build replaces
the sidebar with a contextual floating value keypad. It captures a key at pointer
down before egui can treat it as an outside click, restores the editor owner and
delivers the key on release. The regression test verifies capture, value replacement
and retained focus with an actual keypad pointer press/release sequence.
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
- This contextual-keypad APK has not yet been tested on the physical tablet. Earlier branch builds passed Galaxy Tab S7+ navigation, S Pen hover/selection and palm-rejection testing. Emulator, hardware keyboard, simultaneous pen/finger, SAF save/reopen and rotation/resume tests remain incomplete.

## Implementation

Typed per-pointer routing precedes egui synthesis. Captured viewport fingers cannot select geometry; pen contacts cannot join finger gestures. Gestures calculate independent deltas and centroid/span ratios, handle cancellation and contact transitions, and use the shared camera command path. Sketch mode pans to preserve the sketch plane. A bounded transport preserves ownership even on queue overflow.

The five-column keypad is now a compact, titleless floating window that exists only while an editor owns keyboard focus; it does not reserve viewport width. Its numeric page includes arithmetic, units, editing controls, ✓ commit and ✕ cancel, with an ABC page for text. Key pointer events are removed from the normal UI stream before focus processing, then delivered to the retained editor. Both winit and Java automatic IME paths remain suppressed. The actual-button regression verifies capture, focus retention and replacement of selected text.

Main changed files: `vendor/winit/src/platform/android_cad.rs`, `vendor/winit/src/platform/android.rs`, `vendor/winit/src/platform_impl/android/mod.rs`, `apps/solidcraft-android/src/lib.rs`, `src/keypad.rs`, `src/finger_gestures.rs`, Android Cargo.toml and Cargo.lock, `crates/ui-egui/src/navigation.rs`, `lib.rs`, `viewport.rs`, input/keypad test harnesses, and the template synchronization script. The initial branch commit also updates the Activity's focus policy. Four existing Android bridge files received formatter-only changes. No geometry-kernel implementation changes.

## Installation and tablet testing

Keep the original SolidCraft installed. Install this APK and launch **SolidCraft PenTouch Test**; it has independent files/preferences. Export a copy of a design through SAF if needed.

For this build, focus a dimension and verify that the floating pad appears without shrinking the viewport. Enter `-12.5`, use backspace and re-edit, then tap ✓ and confirm the committed value. Reopen and tap ✕ to confirm cancellation; try arithmetic and the ABC page. Recheck pen select/sketch/hover and one-/two-finger navigation, then rotate, background/resume and save/reopen a copied design.

Full script and architecture: https://github.com/thesuperced/solidcraftAndroid/blob/feat/android-pen-touch-keypad/docs/android-pen-touch-keypad.md

This is a test build awaiting tablet acceptance. OS/OEM palm filtering can suppress simultaneous pen/finger streams; the software cannot establish that capability without hardware evidence.
