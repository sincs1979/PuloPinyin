#!/usr/bin/env bash
# 部落输入法 Linux 一条命令安装（在 Ubuntu/Debian 本机编译 fcitx5 插件）。
#
#   curl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-linux.sh | bash
# 或克隆后：
#   ./安装-linux.sh
set -euo pipefail

REPO_URL="${BULUO_REPO:-https://github.com/sincs1979/PuloPinyin.git}"
CLONE_DIR="${BULUO_SRC:-$HOME/.local/src/PuloPinyin}"

need_cmd() { command -v "$1" >/dev/null 2>&1; }

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "这是 Linux 安装脚本。macOS 请下载仓库根目录的 部落输入法.pkg："
  echo "  https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法.pkg"
  exit 1
fi

install_apt_deps() {
  if ! need_cmd apt-get; then
    echo "非 Debian/Ubuntu：请自行安装 fcitx5、fcitx5 开发头文件、cmake、pkg-config、git、curl、g++，再重新运行。"
    return 0
  fi
  local pkgs=()
  need_cmd git || pkgs+=(git)
  need_cmd curl || pkgs+=(curl)
  need_cmd cmake || pkgs+=(cmake)
  need_cmd pkg-config || pkgs+=(pkg-config)
  need_cmd g++ || pkgs+=(build-essential)
  need_cmd python3 || pkgs+=(python3)
  dpkg -s fcitx5 >/dev/null 2>&1 || pkgs+=(fcitx5)
  dpkg -s fcitx5-dev >/dev/null 2>&1 || pkgs+=(fcitx5-dev)
  dpkg -s extra-cmake-modules >/dev/null 2>&1 || pkgs+=(extra-cmake-modules)
  if ((${#pkgs[@]})); then
    echo "==> apt: ${pkgs[*]}"
    sudo apt-get update
    sudo apt-get install -y "${pkgs[@]}"
  fi
}

ensure_rust() {
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  if need_cmd rustup; then
    return 0
  fi
  echo "==> installing rustup (toolchain 1.88)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.88.0
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
}

resolve_root() {
  local src="${BASH_SOURCE[0]:-}"
  if [[ -n "$src" && -f "$src" && "$src" != "-" && "$src" != "bash" ]]; then
    local here
    here="$(cd "$(dirname "$src")" && pwd)"
    if [[ -f "$here/scripts/install_linux.sh" ]]; then
      echo "$here"
      return 0
    fi
  fi
  return 1
}

install_apt_deps
ensure_rust

if ROOT="$(resolve_root)"; then
  echo "==> using repo at $ROOT"
else
  echo "==> cloning $REPO_URL -> $CLONE_DIR"
  mkdir -p "$(dirname "$CLONE_DIR")"
  if [[ -d "$CLONE_DIR/.git" ]]; then
    git -C "$CLONE_DIR" pull --ff-only || true
  else
    rm -rf "$CLONE_DIR"
    git clone --depth 1 "$REPO_URL" "$CLONE_DIR"
  fi
  ROOT="$CLONE_DIR"
fi

chmod +x "$ROOT/scripts/install_linux.sh"
"$ROOT/scripts/install_linux.sh"

echo
echo "—— 还差一步 ——"
echo "1. 重启输入法：  fcitx5 -r"
echo "2. 打开配置：    fcitx5-configtool"
echo "3. 启用「部落输入法」，输入 zhongguo 再按空格，应得「中国」。"
