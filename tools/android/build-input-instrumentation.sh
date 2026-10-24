#!/usr/bin/env bash
set -euo pipefail
sdk=${ANDROID_HOME:?Set ANDROID_HOME}
bt=${ANDROID_BUILD_TOOLS:-$sdk/build-tools/35.0.0}
key=${APK_TEST_KEYSTORE:?Use the same test key that signed the application}
: "${APK_SIGNING_PASSWORD:?Set APK_SIGNING_PASSWORD}"
export APK_SIGNING_PASSWORD
output=$(realpath -m "${1:?Supply a new test APK path}")
test ! -e "$output"
root=$(cd -- "$(dirname -- "$0")/instrumentation" && pwd)
staging=$(mktemp -d /tmp/solidcraft-instrumentation.XXXXXX)
trap 'rm -rf -- "$staging"' EXIT
mkdir -p "$staging/classes" "$staging/dex"
javac -source 8 -target 8 -bootclasspath "$sdk/platforms/android-35/android.jar" -d "$staging/classes" "$root/InputReplay.java"
jar cf "$staging/classes.jar" -C "$staging/classes" .
"$bt/d8" --min-api 28 --lib "$sdk/platforms/android-35/android.jar" --output "$staging/dex" "$staging/classes.jar"
"$bt/aapt" package -f -M "$root/AndroidManifest.xml" -I "$sdk/platforms/android-35/android.jar" -F "$staging/test.apk"
(cd "$staging/dex" && zip -q -j "$staging/test.apk" classes.dex)
"$bt/zipalign" -f 4 "$staging/test.apk" "$staging/aligned.apk"
"$bt/apksigner" sign --ks "$key" --ks-pass env:APK_SIGNING_PASSWORD --out "$output" "$staging/aligned.apk"
"$bt/apksigner" verify "$output"
