#!/usr/bin/env bash
# Build the portable engine + optional fcitx5 addon into ~/.local.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
# Relocate a Rust staticlib into fcitx5's .so.
export RUSTFLAGS="${RUSTFLAGS:-} -C relocation-model=pic"

PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}/buluo-ime"

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

if ! need_cmd cargo; then
  echo "cargo not found. Install Rust first:" >&2
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
  echo "  source \"\$HOME/.cargo/env\"" >&2
  exit 1
fi

echo "==> building engine + ffi + dict-compiler"
cargo build --release -p engine -p engine-ffi -p dict-compiler
cargo test -p engine -p engine-ffi

ensure_dict() {
  if [[ -f resources/system.dict ]]; then
    return 0
  fi
  echo "==> compiling lexicon"
  if python3 scripts/build_dict.py && [[ -f data/system.tsv ]]; then
    "$CARGO_TARGET_DIR/release/dict-compiler" --system data/system.tsv -o resources/system.dict
  elif [[ -f data/builtin.tsv ]]; then
    echo "==> rime-ice download skipped; compiling builtin.tsv"
    "$CARGO_TARGET_DIR/release/dict-compiler" --system data/builtin.tsv -o resources/system.dict
  else
    echo "no lexicon sources found" >&2
    return 1
  fi
}

ensure_dict

mkdir -p "$DATA" "$PREFIX/share/fcitx5/buluo"
cp resources/system.dict "$DATA/system.dict"
cp resources/system.dict "$PREFIX/share/fcitx5/buluo/system.dict" 2>/dev/null || true
echo "==> lexicon -> $DATA/system.dict"

if ! need_cmd cmake; then
  echo
  echo "cmake not found. Engine library is at:"
  echo "  $CARGO_TARGET_DIR/release/libbuluo_engine.a"
  echo "  $CARGO_TARGET_DIR/release/libbuluo_engine.so  (if the linker produced a cdylib)"
  echo "Install fcitx5 + cmake + pkg-config, then re-run this script to build the addon."
  exit 0
fi

if ! pkg-config --exists Fcitx5Core 2>/dev/null && ! pkg-config --exists fcitx5 2>/dev/null; then
  echo
  echo "Fcitx5Core not found (need fcitx5-dev / fcitx5-devel)."
  echo "Engine still works: cargo test -p engine"
  echo "Addon sources: linux/fcitx5/"
  echo "Next: sudo apt install fcitx5 fcitx5-dev cmake pkg-config  # or dnf/pacman equivalent"
  echo "      ./scripts/install_linux.sh"
  echo "      fcitx5-configtool  # enable 部落输入法"
  exit 0
fi

BUILD="${ROOT}/dist/fcitx5-build"
cmake -S linux/fcitx5 -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX="$PREFIX" \
  -DBULUO_ENGINE_LIB="$CARGO_TARGET_DIR/release/libbuluo_engine.a"
cmake --build "$BUILD"
cmake --install "$BUILD"

echo
echo "Installed fcitx5 addon to $PREFIX"
echo "  lib:  $PREFIX/lib/fcitx5/buluo.so"
echo "  conf: $PREFIX/share/fcitx5/addon/buluo.conf"
echo "  im:   $PREFIX/share/fcitx5/inputmethod/buluo.conf"
echo "Restart fcitx5, then enable 部落输入法 in fcitx5-configtool."
echo "  fcitx5 -r"
echo "  fcitx5-configtool"
