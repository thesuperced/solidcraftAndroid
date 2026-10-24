#!/usr/bin/env bash
# Verify installation constraints, not just the cryptographic signature.
set -euo pipefail
apk=${1:?Usage: verify-test-apk.sh APK}
build_tools=${ANDROID_BUILD_TOOLS:-${ANDROID_HOME:?Set ANDROID_HOME}/build-tools/35.0.0}

method=$(unzip -lv "$apk" resources.arsc | awk '$NF == "resources.arsc" { print $2 }')
if [[ "$method" != Stored ]]; then
    echo "ERROR: resources.arsc must be uncompressed (Stored) for Android 11+ installation; found: $method" >&2
    exit 1
fi
"$build_tools/zipalign" -c -P 16 4 "$apk"
"$build_tools/apksigner" verify --verbose --min-sdk-version 28 "$apk"
"$build_tools/aapt" dump badging "$apk" | awk '/^(package:|sdkVersion:|targetSdkVersion:|launchable-activity:|native-code:)/'
echo "PASS: resource storage, alignment, signature and manifest checks"
