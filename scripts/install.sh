#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

"$ROOT/scripts/assemble_macos_app.sh"

APP_NAME="BuluoIME.app"
DEST="${HOME}/Library/Input Methods/${APP_NAME}"
STAGE="${ROOT}/dist/${APP_NAME}"
APPEX_REL="Contents/PlugIns/BuluoIME.appex"

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
