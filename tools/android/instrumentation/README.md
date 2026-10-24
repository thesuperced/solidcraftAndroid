# Native Android input replay

`InputReplay` creates and injects real Android MotionEvents with stable IDs, mixed
stylus/finger tools, reordered pointer indices, batched history and cancellation.
It queries the app's authenticated `android.input` diagnostic method and asserts
that the pen and UI control have independent owners, history reaches the native
router, and cancel releases every pointer. It does not certify physical S Pen
latency or Samsung palm filtering.

Build with JDK 21 and Android SDK 35:

```sh
export APK_TEST_KEYSTORE=/path/to/the-app-test-key.keystore
export APK_SIGNING_PASSWORD=your-local-password
bash tools/android/build-input-instrumentation.sh /tmp/solidcraft-input-tests.apk
```

Install the app and the instrumentation APK signed with the same key. Open a blank
test document, save any other work, and enable the app's existing loopback control
server (`SOLIDCRAFT_CONTROL_PORT=9977` in its `env.txt`). Use that launch's control
token (or an explicitly configured local test token). Release all physical fingers
and the pen before running:

```powershell
.\adb.exe install -r .\solidcraft-input-instrumentation.apk
.\adb.exe shell am instrument -w -e port 9977 -e token YOUR_CURRENT_CONTROL_TOKEN ai.storyteller.solidcraft.inputtests/ai.storyteller.solidcraft.inputtests.InputReplay
```

Expect a `PASS` stream and result code `-1`. The test ends by canceling its pen
transaction, restoring the blank test document. Run the physical acceptance cases
in `docs/android-pen-first-architecture.md` separately. This harness was compiled
and signed in the build workspace; a device execution result must be recorded by
the person running it. No connected Android device is assumed.
