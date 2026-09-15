#!/usr/bin/env bash
# 部落输入法 macOS 安装（curl 下载，不带隔离属性，避开 Gatekeeper）。
#
#   curl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-macos.sh | bash
# 已克隆仓库：
#   ./安装-macos.sh
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "这是 macOS 安装脚本。Linux 请用："
  echo "  curl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-linux.sh | bash"
  exit 1
fi

TARBALL_URLS=(
  "${BULUO_MACOS_TARBALL:-}"
  "https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/buluo-ime-macos.tar.gz"
  "https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/%E9%83%A8%E8%90%BD%E8%BE%93%E5%85%A5%E6%B3%95-macos.tar.gz"
)

download_tarball() {
  local dest="$1"
  local url
  for url in "${TARBALL_URLS[@]}"; do
    [[ -z "$url" ]] && continue
    echo "==> 下载 $url"
    if curl --http1.1 --fail --location --retry 5 --retry-all-errors --retry-delay 1 \
      --connect-timeout 20 --max-time 300 \
      -o "$dest" "$url"; then
      return 0
    fi
    echo "    失败，试下一个地址…" >&2
  done
  echo "下载安装包失败。请检查网络后重试。" >&2
  return 1
}
SRC="${BASH_SOURCE[0]:-}"
HERE=""
if [[ -n "$SRC" && "$SRC" != "bash" && -f "$SRC" ]]; then
  HERE="$(cd "$(dirname "$SRC")" && pwd)"
fi

APP=""
if [[ -n "$HERE" && -d "$HERE/部落输入法.app" ]]; then
  APP="$HERE/部落输入法.app"
elif [[ -n "$HERE" && -d "$HERE/dist/BuluoIME.app" ]]; then
  APP="$HERE/dist/BuluoIME.app"
elif [[ -n "$HERE" && -d "$HERE/BuluoIME.app" ]]; then
  APP="$HERE/BuluoIME.app"
fi

TMP=""
cleanup() {
  if [[ -n "$TMP" && -d "$TMP" ]]; then
    rm -rf "$TMP"
  fi
}
trap cleanup EXIT

if [[ -z "$APP" ]]; then
  TMP="$(mktemp -d /tmp/buluo-ime.XXXXXX)"
  echo "==> 下载安装包"
  download_tarball "$TMP/buluo-ime-macos.tar.gz"
  xattr -cr "$TMP/buluo-ime-macos.tar.gz" 2>/dev/null || true
  tar -xzf "$TMP/buluo-ime-macos.tar.gz" -C "$TMP"
  if [[ -d "$TMP/部落输入法.app" ]]; then
    APP="$TMP/部落输入法.app"
  elif [[ -d "$TMP/BuluoIME.app" ]]; then
    APP="$TMP/BuluoIME.app"
  else
    echo "压缩包里没有 部落输入法.app" >&2
    exit 1
  fi
fi

xattr -cr "$APP" 2>/dev/null || true

PAYLOAD=""
if [[ -n "$HERE" && -f "$HERE/scripts/install_macos_payload.sh" ]]; then
  PAYLOAD="$HERE/scripts/install_macos_payload.sh"
elif [[ -n "$HERE" && -f "$HERE/install_macos_payload.sh" ]]; then
  PAYLOAD="$HERE/install_macos_payload.sh"
fi

if [[ -n "$PAYLOAD" ]]; then
  bash "$PAYLOAD" "$APP"
else
  # Piped from curl: payload is inside the tarball next to the app.
  if [[ -f "$(dirname "$APP")/install_macos_payload.sh" ]]; then
    bash "$(dirname "$APP")/install_macos_payload.sh" "$APP"
  else
    # Minimal copy if the tarball only has the .app.
    DEST="${BULUO_IME_DEST:-$HOME/Library/Input Methods/BuluoIME.app}"
    mkdir -p "$(dirname "$DEST")"
    rm -rf "$DEST"
    cp -R "$APP" "$DEST"
    xattr -cr "$DEST" 2>/dev/null || true
    echo "Installed to: $DEST"
  fi
fi

if [[ -z "${BULUO_IME_DEST:-}" ]]; then
  echo
  echo "系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +"
  echo "搜索「部落」或「Buluo」并添加（不要只翻简体中文）。"
  echo "先切到别的输入法，再切回部落。输入 zhongguo，空格，应得「中国」。"
fi
