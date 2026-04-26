# Debug Assistant

![version](https://img.shields.io/badge/version-0.1.2-blue)
![platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![license](https://img.shields.io/badge/license-MIT-green)

运行在终端里的 MCU 调试和固件下载工具，面向 STM32 常见开发流程：串口调试、USART ISP 下载、JTAG 下载和 SWD 下载。全键盘操作，不依赖 GUI 框架。

![主界面](images/主界面.png)

---

## 功能概览

| 入口 | 用途 |
|------|------|
| **Serial Monitor** | 串口收发调试终端，支持 ASCII / HEX / BOTH 显示、发送历史、日志导出 |
| **USART ISP Flash** | 通过 STM32 ROM Bootloader 下载 `.bin` / `.hex` 固件，支持手动和自动进入 Bootloader |
| **JTAG Flash** | 通过调试探针使用 JTAG 协议烧录固件，支持芯片预设和手动输入 |
| **SWD Flash** | 通过调试探针使用 SWD 协议烧录固件，支持 Under Reset 连接模式 |

---

## 快速开始

**前置条件**

- Rust 稳定版工具链（`rustup` 安装）
- 串口驱动或探针驱动已就位，系统能识别设备

**构建并运行**

```bash
# 开发版本
cargo run

# 发布版本（推荐日常使用）
cargo run --release
```

**操作流程**

1. 启动后用 `↑ / ↓` 在首页选择协议入口。
2. 按 `Enter` 进入工作台。
3. 在 Setup 面板配置好参数后按 `Enter` 连接或开始烧录。
4. 按 `Esc` 返回上一层，`Ctrl+C` 随时退出。

---

## Serial Monitor

串口通信终端。进入后左侧是 Setup 面板（连接配置和状态），右侧是流量监视区，底部是发送行。

**使用流程**

1. 按 `Tab` 或 `F2` 将焦点切换到 Setup 面板。
2. `↑ / ↓` 导航字段，`← / →` 切换选项，`R`（在 Port 字段）刷新串口列表。
3. 配置好 Port / Baud / Data / Stop / Parity / Flow 后按 `Enter` 连接。
4. `Tab` 切到 Send 面板输入发送内容，`Enter` 发送。
5. `Tab` 切到 Receive 面板用方向键滚动日志。
6. `F6` 复制日志，`F7` 保存日志到 `logs/`。

**支持能力**

- 波特率预设：1200 / 2400 / 4800 / 9600 / 19200 / 38400 / 57600 / 115200 / 230400 / 460800 / 921600
- ASCII、HEX、BOTH 三种显示模式（`F4` 切换）
- 文本发送和 HEX 发送（`Ctrl+H` 切换）
- 换行后缀：None / CR / LF / CRLF（`Ctrl+N` 切换）
- 发送历史（`↑ / ↓` 浏览）
- 自动滚动开关（`F5`）
- 收发字节统计和时间戳
- 串口异常时自动断开并显示状态

---

## USART ISP Flash

通过 STM32 USART Bootloader（AN3155 协议）下载固件，不需要调试探针。

**使用流程**

1. 选择串口和波特率。
2. 选择 Boot 模式。
3. 输入 `.bin` 或 `.hex` 固件路径。
4. 按 `Enter` 开始下载。

**Boot 模式**

| 模式 | 说明 |
|------|------|
| **Manual** | 用户手动让芯片进入 Bootloader（BOOT0 拉高后复位） |
| **Auto** | 软件通过 RTS → BOOT0、DTR → RESET 自动控制进入 Bootloader |

> 如果 Serial Monitor 正在占用同一个串口，ISP 下载前会自动断开，完成后自动恢复。

---

## JTAG Flash

通过调试探针使用 JTAG 协议烧录固件。

**使用流程**

1. 选择 `Probe`，按 `R` 可刷新探针列表。
2. 设置 `Speed`（kHz）。
3. 按需开启 `Verify`（烧录后校验）和 `Reset`（烧录完成后复位运行）。
4. 下载 `.bin` 时确认 `Base` 地址（STM32 内部 Flash 通常为 `0x08000000`）。
5. 选择 `Preset`，或在 `Chip` 字段手动输入 probe-rs 目标名称。
6. 输入固件路径，按 `Enter` 开始烧录。

---

## SWD Flash

通过调试探针使用 SWD 协议烧录固件，流程与 JTAG 基本相同，多一个连接模式选项。

| 连接模式 | 说明 |
|----------|------|
| **Normal** | 普通附加，适合大多数情况 |
| **Under Reset** | 在复位状态下附加，用于普通模式无法连接时 |

---

## 键盘快捷键

### 全局

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 选择项目 / 滚动 |
| `Enter` | 确认 / 打开 / 开始 |
| `Esc` | 返回上一层 |
| `Ctrl+C` | 退出程序 |

### Serial Monitor

| 按键 | 功能 |
|------|------|
| `Tab` | 循环切换焦点：Setup → Receive → Send |
| `Shift+Tab` | 反向切换焦点 |
| `F1` | 打开帮助 |
| `F2` | 跳到 Setup 面板 |
| `F3` | 清空接收日志 |
| `F4` | 切换显示模式（ASCII → HEX → BOTH） |
| `F5` | 切换自动滚动 |
| `F6` | 复制日志到剪贴板 |
| `F7` | 保存日志到 `logs/` |
| `Ctrl+D` | 断开连接 |

**Setup 面板（Tab / F2 进入）**

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 导航字段 |
| `← / →` | 修改当前字段值 |
| `R` | 刷新串口列表（焦点在 Port 字段时） |
| `Enter` | 应用配置并连接 / 重连 |
| `Ctrl+D` | 仅断开连接 |
| `Esc` | 丢弃修改，焦点回到 Send |

**Send 面板**

| 按键 | 功能 |
|------|------|
| `Enter` | 发送 |
| `↑ / ↓` | 浏览发送历史 |
| `← / →` | 移动光标 |
| `Home / End` | 光标跳首 / 尾 |
| `Backspace / Del` | 删除字符 |
| `Ctrl+H` | 切换 HEX 发送模式 |
| `Ctrl+N` | 切换换行后缀（None → CR → LF → CRLF） |

### Flash 工作台

适用于 USART ISP、JTAG 和 SWD。

**配置阶段**

| 按键 | 功能 |
|------|------|
| `↑ / ↓` 或 `Tab / Shift+Tab` | 切换字段 |
| `← / →` | 修改选项 |
| `R` | 刷新探针列表（在 Probe 字段） |
| `F6` | 复制日志 |
| `F7` | 保存日志 |
| `Enter` | 开始烧录 |
| `Esc` | 返回上一层 |

**烧录进行中**

| 按键 | 功能 |
|------|------|
| `↑ / ↓ / PgUp / PgDn / Home / End` | 滚动操作日志 |
| `F6` | 复制日志 |
| `F7` | 保存日志 |
| `Esc` | 第一次：提示取消；第二次：请求停止 |

---

## 日志文件

按 `F7` 保存的日志写入程序运行目录下的 `logs/` 文件夹：

```
logs/serial-20260426-153012.log
logs/flasher-20260426-153045.log
```

---

## 注意事项

- `.hex` 文件自带地址，无需关心 Base 地址。
- `.bin` 文件不含地址，必须确认 `Base` 正确；STM32 内部 Flash 起始地址通常为 `0x08000000`。
- JTAG / SWD 的 `Chip` 名称需要是 probe-rs 可识别的目标名称，不确定时优先使用 `Preset`。
- SWD 无法附加时，尝试切换到 `Under Reset` 模式。
- 探针列表为空时，检查驱动、USB 连接，以及是否被其他工具占用。

---

## 构建

```bash
# debug
cargo build

# release
cargo build --release
```

产物路径（Windows）：

```
target/debug/debug-assistant.exe
target/release/debug-assistant.exe
```

---

## 依赖

| 库 | 用途 |
|----|------|
| [ratatui](https://github.com/ratatui/ratatui) | 终端界面框架 |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 跨平台终端 I/O |
| [serialport](https://github.com/serialport/serialport-rs) | 串口通信 |
| [probe-rs](https://github.com/probe-rs/probe-rs) | JTAG / SWD 调试探针通信 |
| [ihex](https://github.com/mciantyre/ihex) | Intel HEX 文件解析 |
| [chrono](https://github.com/chronotope/chrono) | 时间戳 |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | Unicode 宽字符处理 |
| [arboard](https://github.com/1Password/arboard) | 系统剪贴板 |
| [anyhow](https://github.com/dtolnay/anyhow) | 错误处理 |

---

## 许可证

MIT
