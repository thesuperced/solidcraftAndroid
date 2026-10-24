# Android test APK

Download [SolidCraft PenTouch Test APK — installation fix](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-pentouch-test-installfix-arm64.apk).

Arm64 Android 9 or newer. Installs beside the original SolidCraft as **SolidCraft PenTouch Test**.

Built from commit `a5681007839488288be5b5ae56d4b3d06d0d11c9`.
SHA-256: `55c22cb94f7fc99f55ad2ab7145fafb9b2b27ff2358f88f45e51aeb6e36482eb`.

The APK is approximately 38 MiB and is stored directly on this branch. Its native library is compressed for download and extracted by Android on installation. The resource table (`resources.arsc`) is **uncompressed and 4-byte aligned**, as required for Android 11+ installation. The previous download mistakenly compressed that table and has been replaced. Native code, manifest, Java bytecode and resource contents match the original test build. Signing and packaging checks pass; installation on your physical tablet still needs confirmation.

This is a debug-signed experimental APK awaiting tablet testing. See [the test script](../../docs/android-pen-touch-keypad.md).
