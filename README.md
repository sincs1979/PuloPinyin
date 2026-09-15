# 部落输入法

用 Rust 写的极简、极速拼音输入法。macOS / Linux 都可直接下载安装包。Windows 走 TSF，安装包由 GitHub Actions 放到仓库根目录。

全拼、首拼、混输、平翘舌 / r-l 模糊音、个人词频学习。输入法就是输入法。

## macOS：下载安装

仓库根目录两种安装包，不必克隆代码：

- **[部落输入法.pkg](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法.pkg)**：双击安装向导（要管理员密码，写入 `/Library/Input Methods`）。
- **[部落输入法.dmg](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法.dmg)**：打开磁盘映像，双击「安装.command」，装进当前用户的 `~/Library/Input Methods`。

1. 任选一种装好。
2. **系统设置 → 键盘 → 输入法 → 编辑… → 左下角 +**，搜索「部落」或「Buluo」并添加。它**不会**出现在「简体中文」那一栏（那一栏只有苹果自带的拼音/五笔）。
3. 先切到别的输入法，再切回部落。若列表里暂时没有，退出登录再登录一次。
4. 输入 `zhongguo`，空格，应得 **中国**。

从源码打安装包：

```bash
chmod +x scripts/package_macos.sh
./scripts/package_macos.sh
# 生成 部落输入法.pkg 和 部落输入法.dmg（仓库根目录与 dist/）
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

也可在 **[releases/tag/linux](https://github.com/sincs1979/PuloPinyin/releases/tag/linux)** 下载 ASCII 名 `buluo-ime-linux.deb`（GitHub Release 有时会丢掉中文文件名）。

### 从源码三步装（Ubuntu / Debian）

没有预编译包、或想自己编译时：

```bash
# 1. 依赖 + Rust
sudo apt install fcitx5 libfcitx5core-dev cmake pkg-config build-essential git curl extra-cmake-modules python3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. 一条命令（会克隆并装到 ~/.local）
curl -fsSL https://raw.githubusercontent.com/sincs1979/PuloPinyin/main/安装-linux.sh | bash
# 或：git clone https://github.com/sincs1979/PuloPinyin.git && cd PuloPinyin && ./安装-linux.sh

# 3. 启用
fcitx5 -r
fcitx5-configtool   # 启用「部落输入法」
```

Fedora / Arch 从源码装：`dnf install fcitx5-devel cmake` 或 `pacman -S fcitx5 cmake`，再 `./安装-linux.sh`。

macOS 的 `ime` crate 是 InputMethodKit / objc2，Linux 上不要拿它当输入法。

## Windows：可以做，必须走 TSF

Windows 中文输入法 = **Text Services Framework**（`ITfTextInputProcessor` COM DLL），不是普通窗口程序。

**开发不需要 Windows 电脑。** 编译、测引擎、打安装包都在 GitHub Actions 的 `windows-latest` 云主机上做。只有用的人需要一台 Windows。

CI 成功后，安装包会出现在**仓库根目录**（和 `部落输入法.pkg` 并列）：

**[部落输入法-windows.exe](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法-windows.exe)**

也可在 **[releases/tag/windows](https://github.com/sincs1979/PuloPinyin/releases/tag/windows)** 或 **Actions → windows-ci** 下载。根目录还没有该文件，说明 workflow 还没在 `main` 上跑完。

当前是 **TSF 骨架**，不是能打字的输入法：`engine` 已按 Windows MSVC 测；`ime_win.dll` 能编、能写入 `InprocServer32`；完整 TIP、候选窗、向应用程序提交 UTF-16、Authenticode 签名都还没做。装完不能打字是预期。细节见 [`windows/README.md`](windows/README.md)。

和 macOS `.pkg`、Linux `.deb` 共用同一套 Rust `engine`，前端分别是 IMK / fcitx5 / TSF。

## 第一阶段

- 全拼 / 首拼 / 混输
- 拼音切分
- `z/zh` `c/ch` `s/sh` `r/l` 模糊音（只扩搜索，不参与排序）
- 词频排序 + SQLite 学习 + 自动记词
- `system.dict` / `user.dict` / `learned.db`
- 两行 × 五列候选，数字选词，`,` `.` 翻页，Shift 切换中英
- InputMethodKit 集成（macOS）
- fcitx5 起步插件（Linux）
- TSF 骨架（Windows，GitHub Actions 编译安装包）

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
cargo test -p ime-win
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
engine/                     拼音、词库、排序、学习（跨平台，含 Windows）
ime/                        macOS InputMethodKit 前端
linux/engine-ffi/           给 fcitx5 / ibus 用的 C ABI
linux/fcitx5/               fcitx5 起步插件
linux/Dockerfile             在 Ubuntu 容器里打 .deb
windows/ime-win/            Windows TSF COM DLL 骨架
windows/installer/          Inno Setup + 注册脚本（CI 打安装包）
tools/dict-compiler/        TSV / learned.db → 二进制词库
部落输入法.pkg                 macOS 安装包（仓库根目录，可直接下载）
部落输入法.dmg                 macOS 磁盘映像（打开后双击「安装.command」）
部落输入法-linux.deb           Linux .deb（仓库根目录，可直接下载）
部落输入法-linux.tar.gz        Linux 用户目录包（含 install.sh）
部落输入法-windows.exe         Windows 安装包（CI 成功后出现在根目录）
安装-linux.sh                Linux 从源码一条命令安装
scripts/package_macos.sh    打 部落输入法.pkg 和 部落输入法.dmg
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
