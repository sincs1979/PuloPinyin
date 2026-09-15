#!/bin/bash
# Double-click from the 部落输入法.dmg.
# Browser-downloaded copies are blocked by Gatekeeper until quarantine is cleared.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
xattr -cr "$HERE" 2>/dev/null || true

APP="$HERE/部落输入法.app"
if [[ ! -d "$APP" ]]; then
  APP="$HERE/BuluoIME.app"
fi
if [[ ! -d "$APP" ]]; then
  osascript -e 'display alert "找不到 部落输入法.app" message "请打开「终端」运行：\ncurl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-macos.sh | bash"' >/dev/null
  exit 1
fi

if [[ -f "$HERE/install_macos_payload.sh" ]]; then
  bash "$HERE/install_macos_payload.sh" "$APP"
else
  osascript -e 'display alert "缺少安装脚本" message "请打开「终端」运行：\ncurl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-macos.sh | bash"' >/dev/null
  exit 1
fi

if [[ -z "${BULUO_IME_DEST:-}" ]]; then
  osascript -e 'display dialog "已装到 ~/Library/Input Methods。

系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +
搜索「部落」或「Buluo」并添加（不要只翻简体中文）。
先切到别的输入法，再切回部落。
输入 zhongguo，空格，应得「中国」。" buttons {"好"} default button 1 with title "部落输入法"' >/dev/null
fi
