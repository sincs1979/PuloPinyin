#!/bin/bash
# Double-click from the 部落输入法.dmg: copy into ~/Library/Input Methods and register.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
APP="$HERE/部落输入法.app"
if [[ ! -d "$APP" ]]; then
  APP="$HERE/BuluoIME.app"
fi
if [[ ! -d "$APP" ]]; then
  osascript -e 'display alert "找不到 部落输入法.app" message "请从磁盘映像里运行「安装.command」。"' >/dev/null
  exit 1
fi

DEST="${HOME}/Library/Input Methods/BuluoIME.app"
APPEX_REL="Contents/PlugIns/BuluoIME.appex"
LSREG="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"

killall PuloPinyin BuluoIME BuluoIMEHost 2>/dev/null || true
pluginkit -r "${HOME}/Library/Input Methods/BuluoIME.app/Contents/PlugIns/BuluoIME.appex" 2>/dev/null || true
"$LSREG" -u "$HOME/Library/Input Methods/部落输入法.app" 2>/dev/null || true
"$LSREG" -u "$HOME/Library/Input Methods/PuloPinyin.app" 2>/dev/null || true
"$LSREG" -u "$DEST" 2>/dev/null || true

rm -rf "${HOME}/Library/Input Methods/PuloPinyin.app" \
       "${HOME}/Library/Input Methods/部落输入法.app" \
       "$DEST"
mkdir -p "${HOME}/Library/Input Methods"
cp -R "$APP" "$DEST"
xattr -cr "$DEST" 2>/dev/null || true
chmod -R a+rX "$DEST" || true

"$LSREG" -f "$DEST" || true
"$LSREG" -f "${DEST}/${APPEX_REL}" 2>/dev/null || true
pluginkit -a "${DEST}/${APPEX_REL}" 2>/dev/null || true
pluginkit -e use -i com.buluo.inputmethod.pinyin.extension 2>/dev/null || true

if [[ -f "$HERE/register_ime.swift" ]]; then
  swift "$HERE/register_ime.swift" "$DEST" || true
  swift "$HERE/register_ime.swift" "${DEST}/${APPEX_REL}" || true
fi

killall TextInputMenuAgent TextInputSwitcher 2>/dev/null || true

osascript -e 'display dialog "已装到 ~/Library/Input Methods。

系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +
搜索「部落」或「Buluo」并添加（不要只翻简体中文）。
先切到别的输入法，再切回部落。
输入 zhongguo，空格，应得「中国」。" buttons {"好"} default button 1 with title "部落输入法"' >/dev/null
