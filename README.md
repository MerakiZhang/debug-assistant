# Debug Assistant

基于终端的串口调试与 STM32 固件下载工具，使用 Rust 编写。

启动后进入主菜单，按方向键选择工具，按 `Enter` 进入：

![主界面](images/主界面.png)

| 工具 | 用途 |
|------|------|
| **Serial Monitor** | 串口调试终端 |
| **STM32 Flasher** | STM32 固件下载 |

---

## 功能

### Serial Monitor

- 选择串口并连接设备
- 查看串口接收数据
- 发送文本或 HEX 数据
- 支持 ASCII / HEX / BOTH 显示
- 支持发送历史、换行后缀和自动滚动
- 显示收发字节统计和时间戳
- 串口异常时自动断开并提示状态

### STM32 Flasher

支持三种 STM32 固件下载方式。

#### USART ISP

- 通过串口下载 STM32 固件
- 支持 `.bin` / `.hex` 固件
- 支持手动进入 Bootloader
- 支持通过 RTS/DTR 自动控制 BOOT0/RESET
- 下载时显示日志和进度
- 如果串口调试正在占用同一端口，下载前会自动断开，下载后尝试恢复连接

#### JTAG

- 通过调试探针下载 STM32 固件
- 支持 `.bin` / `.hex` 固件
- 支持选择调试探针
- 需要输入目标芯片名称
- 下载时显示日志和进度

#### SWD

- 通过 SWD 调试探针下载 STM32 固件
- 支持 `.bin` / `.hex` 固件
- 支持选择调试探针
- 支持常用 STM32 芯片预设
- 支持手动输入芯片名称
- 支持设置 SWD 通信速度
- 支持普通连接和 Under Reset 连接
- 支持 `.bin` 基地址设置
- 支持烧录后校验
- 支持烧录完成后复位并运行
- 下载时显示日志和进度

---

## 快捷键

### 通用

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 切换选项 |
| `Enter` | 确认 / 开始 |
| `Esc` | 返回 |
| `q` | 退出 |

### Serial Monitor

| 按键 | 功能 |
|------|------|
| `F1` | 打开帮助 |
| `F2` | 打开串口配置 |
| `F3` | 清空接收日志 |
| `F4` | 切换显示模式 |
| `F5` | 切换自动滚动 |
| `Tab` | 切换接收/发送焦点 |
| `Ctrl+D` | 断开连接 |
| `Ctrl+H` | 切换 HEX 发送模式 |
| `Ctrl+N` | 切换换行后缀 |

### STM32 Flasher

| 按键 | 功能 |
|------|------|
| `Tab / Shift+Tab` | 切换配置项 |
| `← / →` | 修改当前选项 |
| `Backspace` | 删除输入字符 |
| `R` | 在探针字段刷新探针列表 |
| `Enter` | 开始下载 |
| `Esc` | 返回上一级 |

下载过程中：

| 按键 | 功能 |
|------|------|
| `↑ / ↓` | 滚动日志 |
| `PgUp / PgDn` | 翻页滚动日志 |
| `Home / End` | 跳到日志顶部/底部 |
| `Esc` | 请求取消当前下载 |
| `q` | 退出程序 |

---

## 构建

需要安装 Rust 稳定版工具链与 Cargo：

```bash
cargo build --release
```

Windows 下生成的可执行文件位于：

```bash
target/release/debug-assistant.exe
```

---

## 运行

开发运行：

```bash
cargo run --release
```

也可以先构建，再直接运行：

```bash
target/release/debug-assistant.exe
```

---

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
| [anyhow](https://github.com/dtolnay/anyhow) | 错误处理 |
| [tui-big-text](https://github.com/joshka/tui-big-text) | 大号标题文字 |

---

## 许可证

MIT
