# Android test APK

Download [SolidCraft PenTouch Test APK — pen-first selection and controls](https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-pen-first-arm64.apk).

Arm64 Android 9 or newer. Installs beside the original SolidCraft as **SolidCraft PenTouch Test**.

Built from source commit `bf64ff30d924cae8fb30db92b1a1d1fb6340fbef`.
SHA-256: `837f5965173128de3d47a0e50349771e105b9f0692127966ea3c7d1c81e7bfdd`.

The APK is approximately 38 MiB and is stored directly on this branch. Its native library is compressed for download and extracted by Android on installation. The resource table (`resources.arsc`) is **uncompressed and 4-byte aligned**, as required for Android 11+ installation. The previous download mistakenly compressed that table and has been replaced. Native code, manifest, Java bytecode and resource contents match the original test build. Signing and packaging checks pass; installation on your physical tablet still needs confirmation.

This is a debug-signed experimental APK awaiting tablet testing. See [the test script](../../docs/android-pen-touch-keypad.md).

This build keeps the accepted immediate one-finger orbit and coalesced two-finger pan/pinch behavior. The permanent left keypad is replaced by a compact floating keypad that appears only while a value/text editor is active. Key touches are captured before the canvas sees them, so they keep the field focused and deliver the selected value; ✓ commits and ✕ cancels.

New: pen add/toggle selection, double-tap body promotion, hold-to-box and overlap picking, off-hand box filters, semantic modifier dock, shared sketch/gizmo/undo paths, and gesture rollback. The keypad sits beside its field. [Validation and remaining device checks](../../docs/android-pen-first-validation.md).

Update on Windows with [update-tablet.ps1](../../tools/android/update-tablet.ps1): downloads the APK, checks SHA-256, force-stops the app, installs with `-r`, and launches it. The signing key and package ID match the previous test build; uninstalling is unnecessary.
