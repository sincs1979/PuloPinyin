# 部落输入法

用 Rust 写的极简、极速拼音输入法。macOS / Linux 都可直接下载安装包。

全拼、首拼、混输、平翘舌 / r-l 模糊音、个人词频学习。输入法就是输入法。

## macOS：下载安装

仓库根目录的 **[部落输入法.pkg](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法.pkg)** 双击即可，不必克隆代码。

1. 下载 `部落输入法.pkg`，打开，按提示安装（需要管理员密码，会写入 `/Library/Input Methods` 并复制到 `~/Library/Input Methods`）。
2. **系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +**，搜索「部落」或「Buluo」并添加。它**不会**出现在「简体中文」那一栏（那一栏只有苹果自带的拼音/五笔）。
3. 先切到别的输入法，再切回部落。若列表里暂时没有，退出登录再登录一次。
4. 输入 `zhongguo`，空格，应得 **中国**。

从源码打安装包：

```bash
chmod +x scripts/package_macos.sh
./scripts/package_macos.sh
# 生成 部落输入法.pkg（仓库根目录与 dist/）
```

开发机装进当前用户（不打 pkg）：

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

学习数据在 `~/Library/Application Support/部落输入法/learned.db`。

## Linux：下载安装

仓库根目录的预编译包（与 macOS 的 `.pkg` 一样，不必克隆代码）：

- **[部落输入法-linux.deb](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法-linux.deb)**（Ubuntu / Debian）
- **[部落输入法-linux.tar.gz](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法-linux.tar.gz)**（任意发行版，用户目录）

需要已有 **fcitx5**。

**Debian / Ubuntu（`.deb`）**

```bash
sudo apt install fcitx5
# 下载仓库根目录的 部落输入法-linux.deb 后：
sudo apt install ./部落输入法-linux.deb
fcitx5 -r
fcitx5-configtool   # 添加 / 启用「部落输入法」
```

**任意发行版（`.tar.gz`）**

```bash
sudo apt install fcitx5   # 或 dnf/pacman 等价包
tar xf 部落输入法-linux.tar.gz
./部落输入法-linux/install.sh
fcitx5 -r
fcitx5-configtool
```

输入 `zhongguo` 再按空格，应得 **中国**。学习数据在 `~/.local/share/buluo-ime/`。

从源码在 Linux 上重打安装包：`./scripts/package_linux.sh`（仓库根目录会生成上述两个文件）。没有 Linux 机器时可用 `./scripts/package_linux_docker.sh`。

Fedora / Arch 从源码装：`dnf install fcitx5-devel cmake` 或 `pacman -S fcitx5 cmake`，再 `./安装-linux.sh`。

macOS 的 `ime` crate 是 InputMethodKit / objc2，Linux 上不要拿它当输入法。

## 第一阶段

- 全拼 / 首拼 / 混输
- 拼音切分
- `z/zh` `c/ch` `s/sh` `r/l` 模糊音（只扩搜索，不参与排序）
- 词频排序 + SQLite 学习 + 自动记词
- `system.dict` / `user.dict` / `learned.db`
- 两行 × 五列候选，数字选词，`,` `.` 翻页，Shift 切换中英
- InputMethodKit 集成（macOS）
- fcitx5 起步插件（Linux）

不做：AI、云词库、皮肤、语音、插件。

## 开发

本机若链接报 `arm64e.x1-macos`，把 SDK 指到 Xcode 自带的 26.5：

```bash
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
export SDKROOT="$DEVELOPER_DIR/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.5.sdk"
export MACOSX_DEPLOYMENT_TARGET=13.0
```

```bash
cargo test -p engine
python3 scripts/build_dict.py
cargo run -p dict-compiler -- --system data/system.tsv -o resources/system.dict
```

| 输入 | 期望 |
| --- | --- |
| `zhongguo` + Space | 中国 |
| `zg` | 能找到 中国 |
| `si` | 同时出现 `si` / `shi` 候选，模糊不降权 |
| 连续选非第一候选 | 重启后该词仍靠前 |

## 仓库

```
engine/                     拼音、词库、排序、学习（跨平台）
ime/                        macOS InputMethodKit 前端
linux/engine-ffi/           给 fcitx5 / ibus 用的 C ABI
linux/fcitx5/               fcitx5 起步插件
linux/Dockerfile             在 Ubuntu 容器里打 .deb
tools/dict-compiler/        TSV / learned.db → 二进制词库
部落输入法.pkg                 macOS 安装包（仓库根目录，可直接下载）
部落输入法-linux.deb           Linux .deb（仓库根目录，可直接下载）
部落输入法-linux.tar.gz        Linux 用户目录包（含 install.sh）
安装-linux.sh                Linux 从源码一条命令安装
scripts/package_macos.sh    打 部落输入法.pkg
scripts/package_linux.sh    打 部落输入法-linux.deb / .tar.gz（须在 Linux 上）
scripts/install_linux.sh     编译引擎并安装 fcitx5 插件到 ~/.local
```

## 快捷键

| 键 | 作用 |
| --- | --- |
| `a–z` | 拼音 |
| `1–9` `0` | 当前页第 1–10 个候选 |
| Space | 当前页第 1 个 |
| `.` / `,` | 候选窗打开时翻页 |
| Shift（松开） | 中 / 英（组字为空时） |
| Esc | 取消组字 |
| Enter | 上屏原始拼音 |
