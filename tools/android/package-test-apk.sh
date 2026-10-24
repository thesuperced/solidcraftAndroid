#!/usr/bin/env bash
# Compress an existing test build without violating Android's resource-table rules.
set -euo pipefail
input=$(realpath "${1:?Usage: package-test-apk.sh INPUT_APK OUTPUT_APK}")
output=$(realpath -m "${2:?Usage: package-test-apk.sh INPUT_APK OUTPUT_APK}")
build_tools=${ANDROID_BUILD_TOOLS:-${ANDROID_HOME:?Set ANDROID_HOME}/build-tools/35.0.0}
key=${APK_TEST_KEYSTORE:?Set APK_TEST_KEYSTORE to your local test keystore}
: "${APK_SIGNING_PASSWORD:?Set APK_SIGNING_PASSWORD for the local test keystore}"
export APK_SIGNING_PASSWORD
if [[ "$input" == "$output" || -e "$output" ]]; then
    echo "Choose a new output filename; existing APKs are not overwritten" >&2
    exit 1
fi
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
staging=$(mktemp -d /tmp/solidcraft-apk.XXXXXX)
trap 'rm -rf -- "$staging"' EXIT
unzip -q "$input" AndroidManifest.xml 'res/*' resources.arsc classes.dex 'lib/*' -d "$staging"
(
    cd "$staging"
    zip -q -9 -r unsigned.apk AndroidManifest.xml res classes.dex lib
    # Since target SDK 30, Android rejects compressed or unaligned resource tables.
    zip -q -0 unsigned.apk resources.arsc
)
"$build_tools/zipalign" -P 16 -f 4 "$staging/unsigned.apk" "$staging/aligned.apk"
"$build_tools/apksigner" sign --ks "$key" --ks-pass env:APK_SIGNING_PASSWORD --v4-signing-enabled false --out "$output" "$staging/aligned.apk"
bash "$script_dir/verify-test-apk.sh" "$output"
