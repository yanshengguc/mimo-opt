# MiMo-OPT 项目交接文档

> 给接手的 AI：读完这份文档就能理解项目全貌，直接开始优化。

---

## 1. 项目是什么

小米 MiMo Token Plan API 的终端聊天工具。目标用户是程序员。

**一句话：** 类似 ChatGPT/Claude 的终端版，但接的是小米 MiMo API，Tokyo Night 配色，简洁不简陋。

---

## 2. 技术栈

| 层 | 技术 | 说明 |
|---|------|------|
| 语言 | Rust 1.95 | edition 2021 |
| 终端 UI | ratatui 0.29 + crossterm 0.28 | 双缓冲 TUI 框架 |
| HTTP | reqwest 0.12 (json, stream) | API 请求 + SSE 流式 |
| 异步 | tokio 1 (full) | 异步运行时 |
| 序列化 | serde + serde_json | JSON 处理 |
| 其他 | scopeguard, unicode-width, futures-util, dirs, anyhow | |

编译命令：`cd C:/test/mimo-opt && cargo build`
Rust 路径：`C:/Users/yansheng/.cargo/bin/`（可能不在 PATH 中，需 `export PATH="$PATH:/c/Users/yansheng/.cargo/bin"`）

---

## 3. 文件结构

```
C:/test/mimo-opt/
├── Cargo.toml                          # 依赖配置
├── src/
│   ├── main.rs                         # 入口：加载配置，检查 api_key，启动 app
│   ├── app.rs                          # 核心：AppState + 事件循环 + 键盘处理 + 异步任务调度
│   ├── config.rs                       # 配置：读写 ~/.config/mimo-opt/config.json
│   ├── api/
│   │   ├── mod.rs                      # MiMoClient：check_api() 探测 + send_message_stream() 流式对话
│   │   └── types.rs                    # 类型：ChatMessage, ChatRequest, StreamEvent, StreamResult
│   └── ui/
│       ├── mod.rs                      # UI 模块入口
│       ├── theme.rs                    # Tokyo Night 配色常量
│       └── draw.rs                     # 界面绘制：标题栏 / 对话区 / 输入框 / 状态栏
├── PROJECT.md                          # 项目需求文档
├── UI_DESIGN.md                        # 界面视觉设计稿
└── HANDOFF.md                          # 本文档
```

---

## 4. 核心数据流

```
用户按键 → app.rs 事件循环 → 发送请求（tokio::spawn）
                                      ↓
                               api/mod.rs 调用 MiMo API
                                      ↓
                               SSE 流式响应 → bytes_stream()
                                      ↓
                               UTF-8 安全解码（处理跨 chunk 分片）
                                      ↓
                               解析 StreamEvent → 发送 StreamResult 到 channel
                                      ↓
app.rs 从 channel 收到 StreamResult → 更新 AppState → ui/draw.rs 渲染
```

### StreamResult 三种结果
- `StreamResult::Token(text)` — 逐 token 返回文本
- `StreamResult::Done { input_tokens, output_tokens }` — 生成完成，含用量统计
- `StreamResult::Error(err)` — API 错误，UI 显示红色错误信息

---

## 5. API 对接详情

### 认证
- Base URL: `https://token-plan-sgp.xiaomimimo.com/anthropic`
- Header: `api-key: tp-{your_key}`
- Header: `anthropic-version: 2023-06-01`
- 格式: Anthropic Messages API 兼容

### 流式事件类型
```json
// message_start — 首个事件，含 input_tokens
{"type": "message_start", "message": {"usage": {"input_tokens": 123}}}

// content_block_delta — 逐 token
{"type": "content_block_delta", "delta": {"type": "text_delta", "text": "你好"}}

// message_delta — 最终事件，含 output_tokens
{"type": "message_delta", "usage": {"output_tokens": 456}}

// message_stop — 结束
{"type": "message_stop"}
```

### 可用模型
| 模型 | 说明 |
|------|------|
| mimo-v2-flash | 极速轻量，日常编码 |
| mimo-v2.5 | 全模态，默认模型 |
| mimo-v2.5-pro | 旗舰，1M 上下文 |
| mimo-v2-omni | 图像/视频/音频 |
| mimo-v2-5-tts | 语音合成 |

### 费率（粗略，以官方为准）
- Input: ¥2/百万 tokens
- Output: ¥8/百万 tokens

---

## 6. 配置文件

自动创建于：`C:\Users\yansheng\AppData\Roaming\mimo-opt\config.json`

```json
{
  "api_key": "tp-xxx",
  "base_url": "https://token-plan-sgp.xiaomimimo.com/anthropic",
  "model": "mimo-v2-flash"
}
```

---

## 7. 界面布局

```
╭─ MiMo-OPT ────────── ⊛ API OK ──────── ⚡ mimo-v2-flash ─╮   ← 标题栏
╰─────────────────────────────────────────────────────────────╯

   你好                                                 ← 用户消息（亮白）
                                                         ← 空行分隔
   你好！有什么可以帮你的？                              ← MiMo 回复（稍暗灰）

   ┌─ rust ───────────────────────────────────────────┐  ← 代码块（待实现）
   │  fn main() { println!("hello"); }                │
   └──────────────────────────────────────────────────┘

   ⠋ thinking...                                       ← 流式等待（待实现）
   ▎                                                   ← 光标

───────────────────────────────────────────────────────────────

   › 输入消息...▎                          Ctrl+Enter ↵  ← 输入框

╭─────────────────────────────────────────────────────────────╮
│  ◉ 3.2K tok  ↓123 ↑456  ￥0.0004    mimo-v2-flash        │  ← 状态栏
╰─────────────────────────────────────────────────────────────╯
```

### 配色（Tokyo Night）
- 标题紫: `#7aa2f7` | 模型青: `#7dcfff` | 用户白: `#c0caf5` | MiMo灰: `#a9b1d6`
- 状态栏数据: `#7dcfff` | 成功绿: `#9ece6a` | 错误红: `#f7768e` | 边框灰: `#565f89`

---

## 8. 当前状态

### 已实现
- [x] MiMo Token Plan API 流式对话（SSE）
- [x] ratatui 终端界面（Tokyo Night 配色）
- [x] 流式逐 token 输出 + spinner
- [x] API 状态探测（标题栏 ⊛ API OK / 错误）
- [x] API 错误通过 StreamResult::Error 传递到 UI
- [x] UTF-8 跨 chunk 安全解码
- [x] token 用量从 API 响应获取（message_start/message_delta）
- [x] 费用计算（按 input/output 分开计费）
- [x] 终端清理保护（scopeguard）
- [x] 输入编辑（Home/End/左右箭头/Delete/Ctrl+A/E）
- [x] Ctrl+Q 退出 / Ctrl+C 中断生成 / Esc 中断生成
- [x] Arc<MiMoClient> 异步任务共享
- [x] unicode-width 正确计算布局宽度
- [x] 配置文件自动创建 + 友好提示

### 已知未实现
- [ ] 代码块语法高亮（theme.rs 中 CODE_KW/CODE_STR/CODE_FN/CODE_COMMENT 已定义颜色）
- [ ] 多会话管理 + 会话持久化
- [ ] 侧边栏（Ctrl+B 呼出）
- [ ] 项目上下文感知（@文件引用）
- [ ] Shell 命令集成
- [ ] 历史输入浏览（↑/↓）
- [ ] 模型切换（/model 命令）
- [ ] 导出会话为 Markdown
- [ ] 前缀缓存优化
- [ ] Tauri 桌面 GUI

### 编译警告（可忽略）
- `theme.rs` 中 `CODE_BG/CODE_KW/CODE_STR/CODE_FN/CODE_COMMENT/SELECTED` 未使用——留给代码高亮功能

---

## 9. 优化方向（按优先级）

### P0：程序员刚需
1. **代码块渲染** — 用 `syntect` 语法高亮，代码块有深色背景 + 圆角框 + 语言标签
2. **多会话 + 持久化** — Ctrl+N 新建，Ctrl+Tab 切换，会话存到 `~/.config/mimo-opt/sessions/`
3. **项目上下文** — 读当前目录结构注入 system prompt，`@文件名` 引用文件内容
4. **Shell 集成** — MiMo 回复中的命令可一键执行，输出回流对话

### P1：效率提升
5. **快捷操作** — ↑/↓ 历史输入、/model 切换模型、/clear 清空、/export 导出
6. **模型路由** — 简单问题 flash，复杂推理 pro，失败自动降级
7. **导出** — /export 导出 Markdown，/copy 复制最后回复

### P2：差异化
8. **前缀缓存优化** — system prompt 固化，消息格式标准化，状态栏显示命中率
9. **MCP 支持** — 接入 MCP server 操作文件/数据库
10. **HTTP API** — `mimo-opt serve --http` 无头模式，CI/CD 集成

---

## 10. 关键实现细节

### 流式完成检测
通过 channel 的 drop 机制：异步任务完成后 `drop(tx)`，主循环从 `token_rx.try_recv()` 收到 `Disconnected` 知道完成。但当前改用了显式的 `StreamResult::Done`，更可靠。

### UTF-8 跨 chunk 处理
```rust
let mut raw_buffer: Vec<u8> = Vec::new();
// 收到 chunk 后 append 到 raw_buffer
// 用 std::str::from_utf8 尝试解码
// 失败时保留未完成字节到下一个 chunk
```

### 异步任务调度
```rust
let client = Arc::clone(client);  // Arc 共享，不 clone 字段
let tx = token_tx.clone();        // channel sender clone
tokio::spawn(async move {
    let result = client.send_message_stream(&messages, tx.clone()).await;
    if let Err(e) = result {
        let _ = tx.send(StreamResult::Error(e.to_string()));
    }
});
```

### 费用计算
```rust
fn calculate_cost(input_tokens: usize, output_tokens: usize) -> f64 {
    let input_cost = input_tokens as f64 / 1_000_000.0 * 2.0;   // ¥2/百万
    let output_cost = output_tokens as f64 / 1_000_000.0 * 8.0;  // ¥8/百万
    input_cost + output_cost
}
```

---

## 11. 快速开始

```bash
# 设置 Rust PATH
export PATH="$PATH:/c/Users/yansheng/.cargo/bin"

# 编译
cd C:/test/mimo-opt
cargo build

# 运行（首次会提示配置 api_key）
cargo run

# 编辑配置
notepad "C:\Users\yansheng\AppData\Roaming\mimo-opt\config.json"
```

---

## 12. 参考项目

| 项目 | 说明 | 可借鉴 |
|------|------|--------|
| DeepSeek-Reasonix | 前缀缓存优先循环 | 缓存分层架构、成本控制 |
| DeepSeek-TUI | Rust+ratatui 终端 Agent | 工具系统、交互模式、LSP |
| MiMo Code | 基于 Claude Code 改造 | TypeScript+Ink 架构参考 |
| MiMo-Skills | 官方技能包 | MCP/技能系统参考 |
