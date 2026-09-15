#!/bin/bash
# Copy a 部落输入法.app into ~/Library/Input Methods and register it.
# Usage: install_macos_payload.sh /path/to/部落输入法.app
set -euo pipefail

APP="${1:-}"
if [[ -z "$APP" || ! -d "$APP" ]]; then
  echo "usage: $0 /path/to/部落输入法.app" >&2
  exit 1
fi
APP="$(cd "$APP" && pwd)"

DEST="${BULUO_IME_DEST:-$HOME/Library/Input Methods/BuluoIME.app}"
APPEX_REL="Contents/PlugIns/BuluoIME.appex"
LSREG="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# register_ime.swift sits next to the .app in the dmg, or in scripts/ in the repo.
REG=""
for cand in \
  "$(dirname "$APP")/register_ime.swift" \
  "$SCRIPT_DIR/register_ime.swift" \
  "$SCRIPT_DIR/../scripts/register_ime.swift"; do
  if [[ -f "$cand" ]]; then
    REG="$cand"
    break
  fi
done

killall PuloPinyin BuluoIME BuluoIMEHost 2>/dev/null || true
pluginkit -r "${HOME}/Library/Input Methods/BuluoIME.app/Contents/PlugIns/BuluoIME.appex" 2>/dev/null || true
"$LSREG" -u "$HOME/Library/Input Methods/部落输入法.app" 2>/dev/null || true
"$LSREG" -u "$HOME/Library/Input Methods/PuloPinyin.app" 2>/dev/null || true
"$LSREG" -u "$DEST" 2>/dev/null || true

dest_dir="$(dirname "$DEST")"
mkdir -p "$dest_dir"
rm -rf "${HOME}/Library/Input Methods/PuloPinyin.app" \
       "${HOME}/Library/Input Methods/部落输入法.app" \
       "$DEST"
cp -R "$APP" "$DEST"
xattr -cr "$DEST" 2>/dev/null || true
chmod -R a+rX "$DEST" || true

"$LSREG" -f "$DEST" || true
"$LSREG" -f "${DEST}/${APPEX_REL}" 2>/dev/null || true
pluginkit -a "${DEST}/${APPEX_REL}" 2>/dev/null || true
pluginkit -e use -i com.buluo.inputmethod.pinyin.extension 2>/dev/null || true

if [[ -n "$REG" ]]; then
  swift "$REG" "$DEST" || true
  swift "$REG" "${DEST}/${APPEX_REL}" || true
fi

killall TextInputMenuAgent TextInputSwitcher 2>/dev/null || true
echo "Installed to: $DEST"
