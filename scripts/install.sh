#!/usr/bin/env bash
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

echo "==> building engine + compiler + IME"
cargo build --release -p engine -p dict-compiler -p ime

echo "==> building system lexicon"
python3 scripts/build_dict.py
echo "==> compiling system.dict"
mkdir -p resources
./target/release/dict-compiler --system data/system.tsv -o resources/system.dict

echo "==> generating icons"
python3 scripts/make_icon.py

APP_NAME="BuluoIME.app"
DEST="${HOME}/Library/Input Methods/${APP_NAME}"
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
cp target/release/BuluoIME "${STAGE}/Contents/MacOS/BuluoIME"
copy_resources "${STAGE}/Contents/Resources"

printf 'XPC!????' > "${STAGE}/${APPEX_REL}/Contents/PkgInfo"
cp resources/Appex-Info.plist "${STAGE}/${APPEX_REL}/Contents/Info.plist"
cp target/release/BuluoIME "${STAGE}/${APPEX_REL}/Contents/MacOS/BuluoIME"
copy_resources "${STAGE}/${APPEX_REL}/Contents/Resources"

IDENTITY="${CODE_SIGN_IDENTITY:-}"
if [[ -z "$IDENTITY" ]]; then
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

echo "==> installing to ${DEST}"
killall PuloPinyin BuluoIME BuluoIMEHost 2>/dev/null || true
pluginkit -r "${HOME}/Library/Input Methods/BuluoIME.app/Contents/PlugIns/BuluoIME.appex" 2>/dev/null || true

LSREG="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
"$LSREG" -u "$HOME/Library/Input Methods/部落输入法.app" 2>/dev/null || true
"$LSREG" -u "$HOME/Library/Input Methods/PuloPinyin.app" 2>/dev/null || true
"$LSREG" -u "$DEST" 2>/dev/null || true

rm -rf "${HOME}/Library/Input Methods/PuloPinyin.app" \
       "${HOME}/Library/Input Methods/部落输入法.app" \
       "$DEST"
mkdir -p "${HOME}/Library/Input Methods"
cp -R "$STAGE" "$DEST"
xattr -cr "$DEST" 2>/dev/null || true

"$LSREG" -f "$DEST"
"$LSREG" -f "${DEST}/${APPEX_REL}" 2>/dev/null || true

echo "==> registering PlugInKit + Text Input Sources"
pluginkit -a "${DEST}/${APPEX_REL}" 2>/dev/null || true
pluginkit -e use -i com.buluo.inputmethod.pinyin.extension 2>/dev/null || true
swift "$ROOT/scripts/register_ime.swift" "$DEST" || true
swift "$ROOT/scripts/register_ime.swift" "${DEST}/${APPEX_REL}" || true

killall TextInputMenuAgent TextInputSwitcher 2>/dev/null || true

echo
echo "==> pluginkit (textinputmethod-services):"
pluginkit -m -v -p com.apple.textinputmethod-services 2>/dev/null | grep -i -E 'buluo|部落' || echo "  (not listed yet)"

echo
echo "Installed to: $DEST"
echo
echo "这次不会出现在「简体中文」下面（那一栏只有苹果自带的拼音/五笔）。"
echo "请先退出登录再登录，然后："
echo "  系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +"
echo "  在左侧列表或搜索框里找「部落」或「Buluo」，不要只翻简体中文。"
echo
echo "Then switch to 部落输入法 and type:  zhongguo  + Space"
