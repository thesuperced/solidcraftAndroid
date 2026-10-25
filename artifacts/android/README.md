# Android control-panel test APK

Download [SolidCraft PenTouch Test — new control panel](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-control-panel-arm64.apk).

Arm64 Android 9 or newer, approximately 38 MiB. Source commit:
`1ccbbd2caa90f26226eb615b98c644fcb8299bb4`.
SHA-256: `26e3a7ecd4aee4454af5a0543c07daf15b7f6e334a697bfd3796ee399cffa7a2`.

This revision replaces the old modifier dock with arrow / CTRL / Cancel / TAB /
Delete / QP / Redo / Undo / Hide / Finish. It includes pen-dragged placement,
left/right docking, a restore tab, seven saved shortcut slots, selection toggle
fixes, context-aware filter reset and German ABC keys `äöüÄÖÜ`.
[Full behaviour and tablet acceptance checklist](../../docs/android-control-panel-testing.md).

Use the [Windows updater](../../tools/android/update-tablet.ps1): download, SHA-256
check, force-stop, `install -r`, launch. The package ID and signing certificate
match the previously installed test build. Do not uninstall.

Passed: 7 native routing/camera tests, 13 shared interaction tests, 6 keypad tests,
strict keypad Clippy, formatting, all 12 WebAssembly checks, ARM64 build and APK
resource storage/alignment/signature/manifest checks. The actual panel was rendered
offscreen and inspected. Full CI still stops at inherited missing asset attributions.

Real S Pen/finger concurrency and installation remain for tablet verification.
This test build allows delivered finger-camera movement during pen contact as
approved; final simultaneous-camera suppression is deferred. No Samsung settings
are changed. Earlier APKs are retained as recovery options.
