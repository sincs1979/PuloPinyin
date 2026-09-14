#!/usr/bin/env bash
# Build the portable engine + optional fcitx5 addon into ~/.local.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}/buluo-ime"

echo "==> building engine + ffi + dict-compiler"
cargo build --release -p engine -p engine-ffi -p dict-compiler
cargo test -p engine -p engine-ffi

if [[ ! -f resources/system.dict ]]; then
  python3 scripts/build_dict.py
  "$CARGO_TARGET_DIR/release/dict-compiler" --system data/system.tsv -o resources/system.dict
fi

mkdir -p "$DATA"
cp resources/system.dict "$DATA/system.dict"
echo "==> lexicon -> $DATA/system.dict"

if ! command -v cmake >/dev/null 2>&1; then
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
