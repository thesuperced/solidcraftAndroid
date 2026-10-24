# Android test APK

Download [SolidCraft PenTouch Test APK — contextual value keypad](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-context-keypad-arm64.apk).

Arm64 Android 9 or newer. Installs beside the original SolidCraft as **SolidCraft PenTouch Test**.

Built from commit `90977ce15b862a046bf6d67f9058ff6753312e46`.
SHA-256: `42b52bbdefe06ee0d27e7536902ccf8b3a74fb791507d018bbe305890cf91d3d`.

The APK is approximately 38 MiB and is stored directly on this branch. Its native library is compressed for download and extracted by Android on installation. The resource table (`resources.arsc`) is **uncompressed and 4-byte aligned**, as required for Android 11+ installation. The previous download mistakenly compressed that table and has been replaced. Native code, manifest, Java bytecode and resource contents match the original test build. Signing and packaging checks pass; installation on your physical tablet still needs confirmation.

This is a debug-signed experimental APK awaiting tablet testing. See [the test script](../../docs/android-pen-touch-keypad.md).

This build keeps the accepted immediate one-finger orbit and coalesced two-finger pan/pinch behavior. The permanent left keypad is replaced by a compact floating keypad that appears only while a value/text editor is active. Key touches are captured before the canvas sees them, so they keep the field focused and deliver the selected value; ✓ commits and ✕ cancels.
