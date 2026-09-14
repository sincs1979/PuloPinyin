#!/usr/bin/env bash
# Build Linux installers at repo root:
#   部落输入法-linux.tar.gz  (install.sh → ~/.local, no root)
#   部落输入法-linux.deb      (apt/dpkg, fcitx5 addon + lexicon)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
# Relocate a Rust staticlib into fcitx5's shared addon.
export RUSTFLAGS="${RUSTFLAGS:-} -C relocation-model=pic"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "package_linux.sh must run on Linux (got $(uname -s))." >&2
  echo "  • On the Linux PC: ./安装-linux.sh" >&2
  echo "  • Docker:           ./scripts/package_linux_docker.sh" >&2
  echo "  • CI:               .github/workflows/linux-package.yml" >&2
  exit 1
fi

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
need cargo
need cmake
need pkg-config
need python3
need tar
need dpkg-deb

if ! pkg-config --exists Fcitx5Core 2>/dev/null; then
  echo "Fcitx5Core.pc not found. Install libfcitx5core-dev (Ubuntu) or fcitx5-dev (Debian)." >&2
  exit 1
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
VERSION="${VERSION:-0.1.0}"
ARCH="$(dpkg --print-architecture 2>/dev/null || uname -m)"
case "$ARCH" in
  x86_64) ARCH=amd64 ;;
esac
MULTIARCH="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || echo x86_64-linux-gnu)"

echo "==> cargo release: engine + ffi + dict-compiler"
cargo build --release -p engine -p engine-ffi -p dict-compiler
cargo test -p engine -p engine-ffi

if [[ ! -f resources/system.dict ]]; then
  echo "==> compiling system.dict"
  if python3 scripts/build_dict.py && [[ -f data/system.tsv ]]; then
    "$CARGO_TARGET_DIR/release/dict-compiler" --system data/system.tsv -o resources/system.dict
  else
    echo "==> rime-ice download failed; compiling builtin.tsv"
    "$CARGO_TARGET_DIR/release/dict-compiler" --system data/builtin.tsv -o resources/system.dict
  fi
fi

ENGINE_LIB="$CARGO_TARGET_DIR/release/libbuluo_engine.a"
if [[ ! -f "$ENGINE_LIB" ]]; then
  echo "missing $ENGINE_LIB" >&2
  exit 1
fi

FCITX_LIBDIR="/usr/lib/${MULTIARCH}/fcitx5"
if [[ ! -d "$FCITX_LIBDIR" ]]; then
  FCITX_LIBDIR="/usr/lib/fcitx5"
fi

BUILD="${ROOT}/dist/fcitx5-build"
cmake -S linux/fcitx5 -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DBULUO_ENGINE_LIB="$ENGINE_LIB" \
  -DBULUO_FCITX_LIBDIR="${FCITX_LIBDIR}" \
  -DBULUO_FCITX_DATADIR="/usr/share/fcitx5"
cmake --build "$BUILD"

SO="$BUILD/buluo.so"
if [[ ! -f "$SO" ]]; then
  echo "missing $SO" >&2
  exit 1
fi

echo "==> staging portable tarball"
STAGE="${ROOT}/dist/linux-stage"
rm -rf "$STAGE"
mkdir -p \
  "$STAGE/lib/fcitx5" \
  "$STAGE/share/fcitx5/addon" \
  "$STAGE/share/fcitx5/inputmethod" \
  "$STAGE/share/buluo-ime"
cp "$SO" "$STAGE/lib/fcitx5/buluo.so"
cp linux/fcitx5/addon/buluo.conf "$STAGE/share/fcitx5/addon/buluo.conf"
cp linux/fcitx5/inputmethod/buluo.conf "$STAGE/share/fcitx5/inputmethod/buluo.conf"
cp resources/system.dict "$STAGE/share/buluo-ime/system.dict"

cat > "$STAGE/install.sh" <<'EOF'
#!/usr/bin/env bash
# Install 部落输入法 fcitx5 addon for the current user (~/.local).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}/buluo-ime"

mkdir -p \
  "$PREFIX/lib/fcitx5" \
  "$PREFIX/share/fcitx5/addon" \
  "$PREFIX/share/fcitx5/inputmethod" \
  "$DATA"

cp "$HERE/lib/fcitx5/buluo.so" "$PREFIX/lib/fcitx5/buluo.so"
cp "$HERE/share/fcitx5/addon/buluo.conf" "$PREFIX/share/fcitx5/addon/buluo.conf"
cp "$HERE/share/fcitx5/inputmethod/buluo.conf" "$PREFIX/share/fcitx5/inputmethod/buluo.conf"
cp "$HERE/share/buluo-ime/system.dict" "$DATA/system.dict"

echo "Installed 部落输入法 → $PREFIX"
echo "  addon: $PREFIX/lib/fcitx5/buluo.so"
echo "  dict:  $DATA/system.dict"
echo
echo "Next:"
echo "  1. Restart fcitx5  (fcitx5 -r   or log out/in)"
echo "  2. fcitx5-configtool → add 部落输入法"
echo "  3. Type zhongguo, Space → 中国"
EOF
chmod 755 "$STAGE/install.sh"

cat > "$STAGE/README.txt" <<EOF
部落输入法 ${VERSION} (Linux / fcitx5)

Requires: fcitx5

  chmod +x install.sh
  ./install.sh

Then restart fcitx5 and enable 部落输入法 in fcitx5-configtool.
EOF

TAR_NAME="部落输入法-linux.tar.gz"
TAR_DIR="${ROOT}/dist/linux-tardir"
rm -rf "$TAR_DIR"
mkdir -p "$TAR_DIR"
cp -a "$STAGE" "$TAR_DIR/部落输入法-linux"
# GNU tar: reproducible-ish, no macOS xattrs
COPYFILE_DISABLE=1 tar -C "$TAR_DIR" -czf "${ROOT}/dist/${TAR_NAME}" 部落输入法-linux
cp "${ROOT}/dist/${TAR_NAME}" "${ROOT}/${TAR_NAME}"
cp "${ROOT}/dist/${TAR_NAME}" "${ROOT}/dist/buluo-ime-linux.tar.gz"

echo "==> staging .deb"
DEB_ROOT="${ROOT}/dist/linux-deb"
rm -rf "$DEB_ROOT"
mkdir -p \
  "$DEB_ROOT/DEBIAN" \
  "$DEB_ROOT${FCITX_LIBDIR}" \
  "$DEB_ROOT/usr/share/fcitx5/addon" \
  "$DEB_ROOT/usr/share/fcitx5/inputmethod" \
  "$DEB_ROOT/usr/share/buluo-ime" \
  "$DEB_ROOT/usr/share/doc/buluo-ime"
cp "$SO" "$DEB_ROOT${FCITX_LIBDIR}/buluo.so"
cp linux/fcitx5/addon/buluo.conf "$DEB_ROOT/usr/share/fcitx5/addon/buluo.conf"
cp linux/fcitx5/inputmethod/buluo.conf "$DEB_ROOT/usr/share/fcitx5/inputmethod/buluo.conf"
cp resources/system.dict "$DEB_ROOT/usr/share/buluo-ime/system.dict"
cat > "$DEB_ROOT/usr/share/doc/buluo-ime/README" <<EOF
部落输入法 ${VERSION}

Restart fcitx5, then enable 部落输入法 in fcitx5-configtool.
Lexicon: /usr/share/buluo-ime/system.dict
User data: ~/.local/share/buluo-ime/
EOF

INSTALLED_SIZE="$(du -sk "$DEB_ROOT" | awk '{print $1}')"
cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: buluo-ime
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: fcitx5
Maintainer: 部落输入法 <sincs1979@gmail.com>
Installed-Size: ${INSTALLED_SIZE}
Homepage: https://github.com/sincs1979/PuloPinyin
Description: 部落输入法 (fcitx5)
 Minimal, fast Pinyin IME. Enable it in fcitx5-configtool after install.
EOF

DEB_NAME="部落输入法-linux.deb"
if dpkg-deb --help 2>&1 | grep -q root-owner-group; then
  dpkg-deb --root-owner-group --build "$DEB_ROOT" "${ROOT}/dist/${DEB_NAME}"
else
  dpkg-deb --build "$DEB_ROOT" "${ROOT}/dist/${DEB_NAME}"
fi
cp "${ROOT}/dist/${DEB_NAME}" "${ROOT}/${DEB_NAME}"
cp "${ROOT}/dist/${DEB_NAME}" "${ROOT}/dist/buluo-ime-linux.deb"

echo
ls -lh "${ROOT}/${TAR_NAME}" "${ROOT}/${DEB_NAME}"
echo "==> ${ROOT}/${TAR_NAME}"
echo "==> ${ROOT}/${DEB_NAME}"
echo "    also in dist/"
