#!/usr/bin/env bash
# Build a double-clickable macOS installer at repo root: 部落输入法.pkg
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
export CODE_SIGN_IDENTITY="${CODE_SIGN_IDENTITY:-Apple Development: sincs1979@gmail.com (D2D7VZWQ3L)}"

"$ROOT/scripts/assemble_macos_app.sh"

APP="${ROOT}/dist/BuluoIME.app"
if [[ ! -d "$APP" ]]; then
  echo "package_macos: expected $APP" >&2
  exit 1
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
VERSION="${VERSION:-0.1.0}"
IDENT="com.buluo.inputmethod.pinyin"
STAGE="${ROOT}/dist/pkg"
ROOT_PAYLOAD="${STAGE}/payload"
SCRIPTS="${STAGE}/scripts"
COMPONENT="${STAGE}/BuluoIME-component.pkg"
DIST_XML="${STAGE}/Distribution.xml"
OUT_ROOT="${ROOT}/部落输入法.pkg"
OUT_DIST="${ROOT}/dist/部落输入法.pkg"

rm -rf "$STAGE"
mkdir -p "$ROOT_PAYLOAD" "$SCRIPTS"

export COPYFILE_DISABLE=1
cp -R "$APP" "$ROOT_PAYLOAD/BuluoIME.app"
find "$ROOT_PAYLOAD" -name '._*' -delete
dot_clean -m "$ROOT_PAYLOAD" 2>/dev/null || true
cp "$ROOT/scripts/pkg/preinstall" "$SCRIPTS/preinstall"
cp "$ROOT/scripts/pkg/postinstall" "$SCRIPTS/postinstall"
cp "$ROOT/scripts/register_ime.swift" "$SCRIPTS/register_ime.swift"
chmod 755 "$SCRIPTS/preinstall" "$SCRIPTS/postinstall"

ARCH="$(uname -m)"
case "$ARCH" in
  arm64) HOST_ARCH="arm64" ;;
  x86_64) HOST_ARCH="x86_64" ;;
  *) HOST_ARCH="arm64,x86_64" ;;
esac

echo "==> pkgbuild component"
pkgbuild \
  --root "$ROOT_PAYLOAD" \
  --identifier "$IDENT" \
  --version "$VERSION" \
  --install-location "/Library/Input Methods" \
  --scripts "$SCRIPTS" \
  --min-os-version 13.0 \
  "$COMPONENT"

cat > "$DIST_XML" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>部落输入法</title>
    <organization>com.buluo</organization>
    <options customize="never" require-scripts="false" hostArchitectures="${HOST_ARCH}"/>
    <welcome file="Welcome.html"/>
    <pkg-ref id="${IDENT}"/>
    <choices-outline>
        <line choice="default">
            <line choice="${IDENT}"/>
        </line>
    </choices-outline>
    <choice id="default"/>
    <choice id="${IDENT}" visible="false">
        <pkg-ref id="${IDENT}"/>
    </choice>
    <pkg-ref id="${IDENT}" version="${VERSION}" onConclusion="none">BuluoIME-component.pkg</pkg-ref>
</installer-gui-script>
EOF

echo "==> productbuild"
productbuild \
  --distribution "$DIST_XML" \
  --package-path "$STAGE" \
  --resources "$ROOT/scripts/pkg/resources" \
  --identifier "$IDENT" \
  --version "$VERSION" \
  "$OUT_DIST"

INSTALLER_ID="$(security find-identity -v | awk -F'"' '/Developer ID Installer|3rd Party Mac Developer Installer/{print $2; exit}')"
if [[ -n "$INSTALLER_ID" ]]; then
  echo "==> signing pkg with $INSTALLER_ID"
  productsign --sign "$INSTALLER_ID" "$OUT_DIST" "${OUT_DIST}.signed"
  mv "${OUT_DIST}.signed" "$OUT_DIST"
else
  echo "==> no Developer ID Installer identity; pkg contains a signed .app (Gatekeeper may warn)"
fi

cp "$OUT_DIST" "$OUT_ROOT"
ls -lh "$OUT_ROOT"
echo "==> installer: $OUT_ROOT"
echo "    also: $OUT_DIST"
