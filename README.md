# 部落输入法

用 Rust 写的极简、极速 macOS 拼音输入法。

全拼、首拼、混输、平翘舌 / r-l 模糊音、个人词频学习。输入法就是输入法。

## 第一阶段

- 全拼 / 首拼 / 混输
- 拼音切分
- `z/zh` `c/ch` `s/sh` `r/l` 模糊音（只扩搜索，不参与排序）
- 词频排序 + SQLite 学习 + 自动记词
- `system.dict` / `user.dict` / `learned.db`
- 两行 × 五列候选，数字选词，`,` `.` 翻页，Shift 切换中英
- InputMethodKit 集成

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

## 安装

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

然后：

系统设置 → 键盘 → 输入法 → 启用 **部落输入法**

输入 `zhongguo`，空格，应得 **中国**。

| 输入 | 期望 |
| --- | --- |
| `zhongguo` + Space | 中国 |
| `zg` | 能找到 中国 |
| `si` | 同时出现 `si` / `shi` 候选，模糊不降权 |
| 连续选非第一候选 | 重启后该词仍靠前 |

学习数据在：

`~/Library/Application Support/部落输入法/learned.db`

## 仓库

```
engine/                 拼音、词库、排序、学习
ime/                    InputMethodKit 前端
tools/dict-compiler/    TSV / learned.db → 二进制词库
data/builtin.tsv        内置兜底词库
data/system.tsv         安装时生成的常用词库（雾凇拼音）
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
