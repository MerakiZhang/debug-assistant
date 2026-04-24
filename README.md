# Debug Assistant

基于终端的串口调试 & STM32 固件下载工具，使用 Rust 编写。

启动后进入主菜单，按方向键选择工具，**Enter** 进入：

| 项目 | 说明 |
|------|------|
| **Serial Monitor** | 串口调试终端 |
| **STM32 Flasher** | STM32 固件下载（USART ISP / JTAG-SWD） |

---

## Serial Monitor — 串口调试

- 串口参数配置：端口、波特率（300–921600）、数据位、停止位、校验位、流控
- 接收显示：ASCII / HEX / 双栏（HEX + ASCII）三种模式，带毫秒时间戳
- 中英文正确显示，无效字节以 `\xNN` 标注
- 行缓冲接收：按换行符聚合，100 ms 超时自动刷新
- 发送：支持发送历史（↑/↓），可追加换行后缀（None / CR / LF / CRLF）
- HEX 发送模式：输入 `48 65 6C 6C 6F` 直接发送字节
- 滚动日志，自动跟随，TX/RX 字节计数

---

## STM32 Flasher — 固件下载

### USART ISP（串口下载）

通过 STM32 系统存储区 Bootloader（AN3155 协议）下载固件，无需外置调试器：

- 支持 `.bin` / `.hex` 格式固件
- 波特率：300–921600（高波特率自动降级至 115200，保证稳定性）
- 启动模式：
  - **Manual**：手动设置 BOOT0=HIGH 并复位芯片
  - **Auto**：通过 RTS→BOOT0、DTR→RESET 自动控制（支持 Standard / Inverted 两种电平配置）
- 流程：同步 → 读取芯片 ID → 全片擦除 → 写入固件（256 字节/块） → 跳转至 0x08000000

### JTAG / SWD（调试探针下载）

通过 probe-rs 支持的调试探针下载固件：

- 支持常见调试器（ST-Link，J-Link，DAP-Link 等）
- 支持 `.bin` / `.hex` 格式固件
- 手动选择调试探针，并输入目标芯片名称（如 `STM32F103C8`）

---

## 构建

需要 Rust 工具链（1.75+）：

```bash
cargo build --release
```

产物位于 `target/release/debug-assistant`（Windows 下为 `debug-assistant.exe`）。

---

## 使用

```bash
cargo run --release
```

启动后进入 Home 主菜单：

1. **↑ / ↓** 选择工具
2. **Enter** 进入

### Serial Monitor 快捷键

#### 全局

| 按键 | 功能 |
|------|------|
| F1 | 打开帮助（任意键关闭） |
| F2 | 串口配置 / 连接 |
| F3 | 清除接收日志 |
| F4 | 切换显示模式（ASCII → HEX → BOTH） |
| F5 | 切换自动滚动 |
| Tab | 切换焦点（接收 ↔ 发送） |
| Ctrl+D | 断开连接 |
| Ctrl+C / q | 退出 |
| Esc | 返回主菜单 |

#### 发送面板

| 按键 | 功能 |
|------|------|
| Enter | 发送 |
| ↑ / ↓ | 浏览发送历史 |
| ← / → | 移动光标 |
| Home / End | 跳至行首 / 行尾 |
| Backspace / Delete | 删除字符 |
| Ctrl+H | 切换 HEX 发送模式 |
| Ctrl+N | 切换换行后缀（None → CR → LF → CRLF） |

#### 接收面板

| 按键 | 功能 |
|------|------|
| ↑ / ↓ | 滚动一行 |
| PgUp / PgDn | 滚动一页 |
| Home / End | 跳至顶部 / 底部 |

#### 配置弹窗

| 按键 | 功能 |
|------|------|
| ↓ / Tab | 下一个配置项 |
| ↑ / Shift+Tab | 上一个配置项 |
| ← / → | 修改当前值 |
| Enter | 应用并连接 |
| Esc | 取消 |

### STM32 Flasher 快捷键

#### 方法选择

| 按键 | 功能 |
|------|------|
| ↑ / ↓ | 选择下载方式 |
| Enter | 进入配置 |
| Esc | 返回主菜单 |

#### 配置界面

| 按键 | 功能 |
|------|------|
| ↓ / Tab | 下一个配置项 |
| ↑ / Shift+Tab | 上一个配置项 |
| ← / → | 修改当前值（端口、波特率、启动模式等） |
| Backspace | 删除文件路径字符 |
| Enter | 开始下载 |
| Esc | 返回方法选择 |

#### 进度界面

| 按键 | 功能 |
|------|------|
| ↑ / ↓ | 滚动日志 |
| Esc | 停止下载并返回配置 |
| q | 退出程序 |

---

## 依赖

| 库 | 用途 |
|----|------|
| [ratatui](https://github.com/ratatui/ratatui) | TUI 框架 |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 终端控制 |
| [serialport](https://github.com/serialport/serialport-rs) | 串口读写 |
| [chrono](https://github.com/chronotope/chrono) | 时间戳 |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | 中文宽字符光标定位 |
| [anyhow](https://github.com/dtolnay/anyhow) | 错误处理 |
| [tui-big-text](https://github.com/joshka/tui-big-text) | 大号标题文字 |
| [probe-rs](https://github.com/probe-rs/probe-rs) | JTAG/SWD 调试探针通信 |
| [ihex](https://github.com/mciantyre/ihex) | Intel HEX 文件解析 |

---

## 许可证

MIT
