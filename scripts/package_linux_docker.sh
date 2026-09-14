#!/usr/bin/env bash
# Build 部落输入法-linux.deb / .tar.gz via Docker (Ubuntu). No-op hint if Docker is missing.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker 不可用。本机若是 macOS，请任选：" >&2
  echo "  1) 在 Linux 电脑执行： curl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-linux.sh | bash" >&2
  echo "  2) 等 GitHub Actions 产出： https://github.com/sincs1979/PuloPinyin/releases/tag/linux" >&2
  exit 1
fi

IMAGE="${BULUO_LINUX_IMAGE:-buluo-ime-linux-pkg}"
docker build -f linux/Dockerfile -t "$IMAGE" "$ROOT"
docker run --rm \
  -v "$ROOT:/src" \
  -w /src \
  -e CARGO_TARGET_DIR=/src/target-linux \
  "$IMAGE" \
  ./scripts/package_linux.sh

ls -lh "$ROOT/部落输入法-linux.deb" "$ROOT/部落输入法-linux.tar.gz"
