# Windows 输入法（TSF）

Windows 上的中文输入法不是普通 GUI 程序，必须做 **Text Services Framework**：进程内 COM DLL，实现 `ITfTextInputProcessor`，注册到 `InprocServer32`，由系统加载。不能做成一个单独的 `.exe` 打字程序。

## 你不需要 Windows 开发机

这台电脑可以一直是 macOS。编译、测试、打安装包都在 **GitHub Actions `windows-latest`**（免费云主机，自带 MSVC）上做。只有**最终用户**需要一台 Windows 来安装、启用输入法。

本地不要装 Visual Studio，也不要交叉编译 TSF。

## 安装包下载位置

CI 成功后，安装包放在**仓库根目录**（和 macOS 的 `部落输入法.pkg` 并列），可直接下：

**[部落输入法-windows.exe](https://github.com/sincs1979/PuloPinyin/raw/main/部落输入法-windows.exe)**

Actions 产物：**Actions → windows-ci**。滚动发布页：**[releases/tag/windows](https://github.com/sincs1979/PuloPinyin/releases/tag/windows)**。

根目录暂时没有该文件，说明 workflow 还没在 `main` 上跑成功。

## 当前状态（诚实）

| 项 | 状态 |
| --- | --- |
| `engine` 在 `x86_64-pc-windows-msvc` 上 `cargo test` | CI 要过（无 macOS-only 代码） |
| TSF COM DLL 骨架（`ime_win.dll`，DllGetClassObject / 注册表） | 能编；**还不是可打字的 TIP** |
| 引擎按键映射、UTF-16 上屏缓冲区、2×5 候选布局 | 源码 + 单测已有 |
| 真 `ITfTextInputProcessor` + `ITfKeyEventSink` | **未做** |
| 候选窗 HWND | **未做** |
| 向焦点控件提交 UTF-16 | **未做**（缺 TSF composition） |
| x86 WoW64 32 位 TIP | **未做**（本轮只打 x64） |
| 安装包 `.exe`（Inno Setup） | CI 打到仓库根目录 |
| Authenticode 签名 | **未做**（SmartScreen 会提示） |
| `.msi`（WiX） | 以后可加；现在用 Inno `setup.exe` |

`DllGetClassObject` 目前返回 `CLASS_E_CLASSNOTAVAILABLE`。安装包仍会写入 CLSID / CTF TIP 键，方便以后接上真实 COM 类。装完如果不能打字，是预期行为，不是引擎坏了。

## 和 macOS / Linux 的关系

同一套 Rust **`engine`**（拼音、词库、排序、学习）：

- **macOS**：`ime/` + InputMethodKit → 根目录 `部落输入法.pkg`
- **Linux**：`linux/engine-ffi` + fcitx5 → 根目录 / Release 的 `.deb` `.tar.gz`
- **Windows**：`windows/ime-win` + TSF COM → 根目录 `部落输入法-windows.exe`

前端不能共用：IMK、fcitx5、TSF 是三种宿主。不要在 Windows 上跑 macOS 的 `ime` crate。

学习数据目录：`%LOCALAPPDATA%\BuluoIME\`（`learned.db` / `user.dict`）。

## 编译 / 注册（在 Windows 上，或 CI 里）

```powershell
# GitHub Actions 已做；真机只为了跑输入法
rustup target add x86_64-pc-windows-msvc
cargo test -p engine --target x86_64-pc-windows-msvc
cargo test -p ime-win --target x86_64-pc-windows-msvc
cargo build --release -p ime-win --target x86_64-pc-windows-msvc
.\windows\installer\build.ps1
```

手动注册（管理员 PowerShell）：

```powershell
.\windows\installer\register.ps1 -DllPath .\target\x86_64-pc-windows-msvc\release\ime_win.dll
# 以后 DLL 实现了 DllRegisterServer：
#   regsvr32 /s .\ime_win.dll
```

注销：`.\windows\installer\unregister.ps1` 或 `regsvr32 /u ime_win.dll`。

系统：**设置 → 时间和语言 → 输入 → 语言** 添加「部落输入法」。在真正的 TIP 完成之前，列表里可能没有、或选了也不能组字。

## 安装包形态

当前 CI 用 **Inno Setup** 打 `部落输入法-windows.exe`（管理员、写 `Program Files\BuluoIME` + 注册表）。等价的 `.msi` 可以用 WiX（`InprocServer32` + CTF TIP 组件），尚未接。

签名：需要 Authenticode 证书后 `signtool sign /tr http://timestamp...`。没有证书时 SmartScreen 会拦截，用户得点「仍要运行」。

## 还剩什么才能当输入法用

1. 用 `windows` crate（或 C++）实现 `ITfTextInputProcessor` / `ITfTextInputProcessorEx`、`ITfKeyEventSink`、`ITfComposition`。
2. `DllGetClassObject` 返回 class factory；`regsvr32` 可自注册。
3. 按键进 `ImeSession`，候选 HWND（2×5），空格/数字把 UTF-16 提交到 `ITfContext`。
4. 可选：32 位 DLL、WiX `.msi`、EV 代码签名。
