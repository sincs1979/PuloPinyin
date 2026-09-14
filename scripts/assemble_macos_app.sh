#!/usr/bin/env bash
# Build and sign dist/BuluoIME.app (no install). Used by install.sh and package_macos.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
if [[ -z "${SDKROOT:-}" ]]; then
  if [[ -d "$DEVELOPER_DIR/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.5.sdk" ]]; then
    export SDKROOT="$DEVELOPER_DIR/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.5.sdk"
  else
    export SDKROOT="$DEVELOPER_DIR/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
  fi
fi
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

export COPYFILE_DISABLE=1

echo "==> building engine + compiler + IME"
cargo build --release -p engine -p dict-compiler -p ime

if [[ ! -f resources/system.dict ]]; then
  echo "==> building system lexicon"
  python3 scripts/build_dict.py
  echo "==> compiling system.dict"
  mkdir -p resources
  "$CARGO_TARGET_DIR/release/dict-compiler" --system data/system.tsv -o resources/system.dict
else
  echo "==> reusing resources/system.dict"
fi

echo "==> generating icons"
python3 scripts/make_icon.py

APP_NAME="BuluoIME.app"
STAGE="${ROOT}/dist/${APP_NAME}"
APPEX_REL="Contents/PlugIns/BuluoIME.appex"

copy_resources() {
  local dest="$1"
  mkdir -p "${dest}/zh-Hans.lproj" "${dest}/en.lproj"
  cp resources/system.dict "${dest}/system.dict"
  cp resources/icon.icns "${dest}/icon.icns"
  cp resources/menu.tiff "${dest}/menu.tiff"
  cp resources/icon.tiff "${dest}/icon.tiff"
  cp resources/zh-Hans.lproj/InfoPlist.strings "${dest}/zh-Hans.lproj/InfoPlist.strings"
  cp resources/en.lproj/InfoPlist.strings "${dest}/en.lproj/InfoPlist.strings"
}

echo "==> assembling ${APP_NAME}"
rm -rf "$STAGE"
mkdir -p "${STAGE}/Contents/MacOS" \
         "${STAGE}/Contents/Resources" \
         "${STAGE}/${APPEX_REL}/Contents/MacOS" \
         "${STAGE}/${APPEX_REL}/Contents/Resources"

printf 'APPL????' > "${STAGE}/Contents/PkgInfo"
cp resources/Info.plist "${STAGE}/Contents/Info.plist"
cp "$CARGO_TARGET_DIR/release/BuluoIME" "${STAGE}/Contents/MacOS/BuluoIME"
copy_resources "${STAGE}/Contents/Resources"

printf 'XPC!????' > "${STAGE}/${APPEX_REL}/Contents/PkgInfo"
cp resources/Appex-Info.plist "${STAGE}/${APPEX_REL}/Contents/Info.plist"
cp "$CARGO_TARGET_DIR/release/BuluoIME" "${STAGE}/${APPEX_REL}/Contents/MacOS/BuluoIME"
copy_resources "${STAGE}/${APPEX_REL}/Contents/Resources"

IDENTITY="${CODE_SIGN_IDENTITY:-Apple Development: sincs1979@gmail.com (D2D7VZWQ3L)}"
if ! security find-identity -v -p codesigning | grep -F -q "$IDENTITY"; then
  IDENTITY="$(security find-identity -v -p codesigning | awk -F'"' '/Apple Development|Developer ID Application/{print $2; exit}')"
fi

APPEX_STAGE="${STAGE}/${APPEX_REL}"
if [[ -n "$IDENTITY" ]]; then
  echo "==> signing with $IDENTITY"
  codesign --force --sign "$IDENTITY" \
    --identifier com.buluo.inputmethod.pinyin.extension \
    --entitlements "$ROOT/resources/Appex.entitlements" \
    "$APPEX_STAGE"
  # Host must NOT be sandboxed. Sandbox + IMK = Launch Services launch-disabled.
  codesign --force --sign "$IDENTITY" \
    --identifier com.buluo.inputmethod.pinyin \
    "$STAGE"
else
  echo "==> no Apple signing identity; ad-hoc (System Settings may hide it)"
  codesign --force --sign - --identifier com.buluo.inputmethod.pinyin.extension "$APPEX_STAGE" >/dev/null
  codesign --force --sign - --identifier com.buluo.inputmethod.pinyin "$STAGE" >/dev/null
fi

echo "==> assembled ${STAGE}"
echo "$STAGE"
