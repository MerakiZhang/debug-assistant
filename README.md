# Debug Assistant

Debug Assistant 是一个运行在终端里的 MCU 调试和固件下载工具，面向 STM32 常见开发流程：串口调试、USART ISP 下载、JTAG 下载和 SWD 下载。

启动后先选择协议入口，再进入对应工作台。所有操作都可以用键盘完成。

![主界面](images/主界面.png)

## 能做什么

| 入口 | 适合场景 |
|------|----------|
| **Serial** | 串口收发调试，或通过 STM32 USART Bootloader 下载固件 |
| **JTAG** | 使用调试探针通过 JTAG 给 STM32 下载固件 |
| **SWD** | 使用调试探针通过 SWD 给 STM32 下载固件 |

## 快速开始

1. 安装探针驱动或串口驱动，确认系统能识别设备。
2. 构建并运行程序。
3. 在首页用 `↑ / ↓` 选择 `Serial`、`JTAG` 或 `SWD`。
4. 按 `Enter` 进入工作台。
5. 填好端口、探针、芯片名和固件路径。
6. 按 `Enter` 开始连接或下载。

## Serial 怎么用

进入 `Serial` 后可以选择两个功能：

| 功能 | 用途 |
|------|------|
| **Serial Monitor** | 普通串口终端，适合看日志、发命令、收发 HEX 数据 |
| **USART ISP Flash** | 通过 STM32 ROM Bootloader 下载 `.bin` 或 `.hex` 固件 |

### Serial Monitor

常用流程：

1. 按 `F2` 打开串口配置。
2. 选择串口号、波特率、数据位、停止位、校验位和流控。
3. 连接后查看接收区日志。
4. 在发送区输入文本或 HEX 数据。
5. 需要保存问题现场时，按 `F6` 复制日志或 `F7` 保存日志。

支持能力：

- ASCII、HEX、BOTH 三种显示模式。
- 文本发送和 HEX 发送。
- 发送历史。
- 换行后缀开关。
- 自动滚动开关。
- 收发字节统计和时间戳。
- 串口异常时自动断开并显示状态。

### USART ISP Flash

常用流程：

1. 选择串口。
2. 选择波特率。
3. 选择 Boot 模式：`Manual` 或 `Auto`。
4. 输入 `.bin` 或 `.hex` 固件路径。
5. 按 `Enter` 开始下载。

Boot 模式说明：

| 模式 | 说明 |
|------|------|
| **Manual** | 用户自己让芯片进入 STM32 Bootloader，例如 BOOT0 拉高后复位 |
| **Auto** | 软件通过 RTS/DTR 控制 BOOT0/RESET，自动尝试进入 Bootloader |

Auto 模式默认线路：

- `RTS -> BOOT0 high`
- `DTR -> RESET low pulse`

如果 Serial Monitor 正在占用同一个串口，USART ISP 下载前会自动断开，下载完成后会尝试恢复串口监视连接。

## JTAG 怎么用

JTAG 工作台用于通过调试探针下载 STM32 固件。它不是完整调试器，不提供断点、单步、寄存器查看等调试功能。

常用流程：

1. 选择 `Probe`。
2. 选择 `Speed`。
3. 按需开启或关闭 `Verify`。
4. 按需开启或关闭 `Reset`。
5. 如果下载 `.bin`，确认 `Base` 是正确的 Flash 起始地址，例如 `0x08000000`。
6. 选择 `Preset`，或手动输入 `Chip`。
7. 输入 `.bin` 或 `.hex` 固件路径。
8. 按 `Enter` 开始下载。

JTAG 支持能力：

- 枚举和选择调试探针。
- 设置 JTAG 通信速度。
- 常用 STM32 芯片预设。
- 手动输入目标芯片名称。
- `.bin` 和 `.hex` 固件下载。
- `.bin` 基地址设置。
- 烧录后校验。
- 烧录完成后 reset + run。
- 下载日志和进度显示。
- 日志复制和保存。

## SWD 怎么用

SWD 工作台用于通过调试探针下载 STM32 固件，配置项比 JTAG 多一个连接模式。

常用流程：

1. 选择 `Probe`。
2. 选择 `Speed`。
3. 选择连接模式：`Normal` 或 `Under Reset`。
4. 按需开启或关闭 `Verify`。
5. 按需开启或关闭 `Reset`。
6. 如果下载 `.bin`，确认 `Base` 是正确的 Flash 起始地址。
7. 选择 `Preset`，或手动输入 `Chip`。
8. 输入 `.bin` 或 `.hex` 固件路径。
9. 按 `Enter` 开始下载。

SWD 支持能力：

- 枚举和选择调试探针。
- 设置 SWD 通信速度。
- 普通连接和 Under Reset 连接。
- 常用 STM32 芯片预设。
- 手动输入目标芯片名称。
- `.bin` 和 `.hex` 固件下载。
- `.bin` 基地址设置。
- 烧录后校验。
- 烧录完成后 reset + run。
- 下载日志和进度显示。
- 日志复制和保存。

## 常用按键

### 全局

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 选择项目或滚动当前区域 |
| `Enter` | 打开、确认或开始 |
| `Esc` | 返回上一层；下载中用于请求取消 |
| `q` | 仅在导航页退出程序 |
| `Ctrl+C` | 任意页面退出；下载中会先请求停止 |

### Serial Monitor

| 按键 | 功能 |
|------|------|
| `F1` | 打开帮助 |
| `F2` | 打开串口配置 |
| `F3` | 清空接收日志 |
| `F4` | 切换显示模式 |
| `F5` | 切换自动滚动 |
| `F6` | 复制串口日志到剪贴板 |
| `F7` | 保存串口日志到 `logs/` |
| `Tab` | 切换接收区和发送区焦点 |
| `Ctrl+D` | 断开连接 |
| `Ctrl+H` | 切换 HEX 发送模式 |
| `Ctrl+N` | 切换换行后缀 |

### Flash 工作台

适用于 `USART ISP Flash`、`JTAG` 和 `SWD`。

| 按键 | 功能 |
|------|------|
| `Tab / Shift+Tab` | 切换配置项 |
| `← / →` | 修改当前选项 |
| `Backspace` | 删除输入字符 |
| `R` | 在探针字段刷新探针列表 |
| `F6` | 复制下载日志到剪贴板 |
| `F7` | 保存下载日志到 `logs/` |
| `Enter` | 开始下载 |
| `Esc` | 返回上一层 |

下载过程中：

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 滚动日志 |
| `PgUp / PgDn` | 翻页滚动日志 |
| `Home / End` | 跳到日志顶部或底部 |
| `F6` | 复制下载日志到剪贴板 |
| `F7` | 保存下载日志到 `logs/` |
| `Esc` | 第一次提示取消，第二次请求停止下载 |
| `Ctrl+C` | 请求停止并退出程序 |

## 日志保存在哪里

按 `F7` 保存日志后，文件会写入项目运行目录下的 `logs/` 文件夹。

日志文件名示例：

```text
logs/serial-20260426-153012.log
logs/flasher-20260426-153045.log
```

## 固件路径和芯片名注意事项

- `.hex` 文件自带地址，通常不需要关心 base address。
- `.bin` 文件不带地址，必须确认 `Base` 正确；STM32 内部 Flash 常见起始地址是 `0x08000000`。
- JTAG/SWD 的 `Chip` 名称需要是 `probe-rs` 识别的目标名称。
- 如果不确定芯片名，优先使用 `Preset`。
- 如果 SWD attach 失败，可以尝试 `Under Reset`。
- 如果探针列表为空，先检查驱动、USB 连接和是否被其他工具占用。

## 构建

需要先安装 Rust 稳定版工具链和 Cargo。

构建 debug 版本：

```bash
cargo build
```

构建 release 版本：

```bash
cargo build --release
```

Windows 下默认产物路径：

```text
target/debug/debug-assistant.exe
target/release/debug-assistant.exe
```

## 运行

开发运行：

```bash
cargo run
```

release 运行：

```bash
cargo run --release
```

或直接运行已构建的可执行文件：

```text
target/release/debug-assistant.exe
```

## 依赖

| 库 | 用途 |
|----|------|
| [ratatui](https://github.com/ratatui/ratatui) | 终端界面 |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 终端输入输出 |
| [serialport](https://github.com/serialport/serialport-rs) | 串口通信 |
| [probe-rs](https://github.com/probe-rs/probe-rs) | JTAG/SWD 调试探针通信 |
| [ihex](https://github.com/mciantyre/ihex) | HEX 文件解析 |
| [chrono](https://github.com/chronotope/chrono) | 时间戳 |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | 中文宽字符处理 |
| [arboard](https://github.com/1Password/arboard) | 系统剪贴板 |
| [anyhow](https://github.com/dtolnay/anyhow) | 错误处理 |

## 许可证

MIT
