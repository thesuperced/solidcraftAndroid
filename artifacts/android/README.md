# Android test APK

Download [SolidCraft PenTouch Test APK — two-finger latency fix](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-pentouch-panfix-arm64.apk).

Arm64 Android 9 or newer. Installs beside the original SolidCraft as **SolidCraft PenTouch Test**.

Built from commit `a5681007839488288be5b5ae56d4b3d06d0d11c9`.
SHA-256: `da5d455e7ef5135ba7800210805f0a47934ac59685338adc03f07558a430ff40`.

The APK is approximately 38 MiB and is stored directly on this branch. Its native library is compressed for download and extracted by Android on installation. The resource table (`resources.arsc`) is **uncompressed and 4-byte aligned**, as required for Android 11+ installation. The previous download mistakenly compressed that table and has been replaced. Native code, manifest, Java bytecode and resource contents match the original test build. Signing and packaging checks pass; installation on your physical tablet still needs confirmation.

This is a debug-signed experimental APK awaiting tablet testing. See [the test script](../../docs/android-pen-touch-keypad.md).

The `panfix` build batches all finger samples queued between rendered frames into one camera update and requests one redraw per Android motion event. This prevents two-finger pan and pinch from replaying an input backlog after the fingers lift.
