# MiMo-OPT 项目交接文档

> 给接手的 AI：读完这份文档就能理解项目全貌，直接开始优化。

---

## 1. 项目是什么

小米 MiMo Token Plan API 的终端聊天工具。目标用户是程序员。

**一句话：** 类似 ChatGPT/Claude 的终端版，但接的是小米 MiMo API，Tokyo Night 配色，简洁不简陋。

**当前状态：** 终端版功能完整，15 项优化全部完成，编译 0 warnings。桌面端（Tauri）已决定跳过，终端版先交付。

---

## 2. 技术栈

| 层 | 技术 | 说明 |
|---|------|------|
| 语言 | Rust 1.95 | edition 2021 |
| 终端 UI | ratatui 0.29 + crossterm 0.28 | 双缓冲 TUI 框架 |
| HTTP | reqwest 0.12 (json, stream) | API 请求 + SSE 流式 |
| 异步 | tokio 1 (full) | 异步运行时 |
| 序列化 | serde + serde_json | JSON 处理 |
| 语法高亮 | syntect 5 (default-fancy) | 代码块着色 |
| 剪贴板 | cli-clipboard 0.4 | 复制代码块 |
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
│   ├── app.rs                          # 核心：AppState + 事件循环 + 键盘处理 + 异步任务调度（~1500 行）
│   ├── config.rs                       # 配置：读写 ~/.config/mimo-opt/config.json + save()
│   ├── file_ops.rs                     # 文件操作：read/write/edit，路径沙箱 + 自动备份
│   ├── session.rs                      # 会话持久化：Session 结构 + JSON 存储
│   ├── api/
│   │   ├── mod.rs                      # MiMoClient：check_api() 探测 + send_message_stream() 流式对话
│   │   └── types.rs                    # 类型：ChatMessage, Content enum, ChatRequest, StreamEvent, StreamResult
│   └── ui/
│       ├── mod.rs                      # UI 模块入口
│       ├── theme.rs                    # Tokyo Night 配色常量
│       └── draw.rs                     # 界面绘制：标题栏 / 对话区 / 输入框 / 状态栏 / 确认弹窗 / 搜索栏
├── OPTIMIZATION.md                     # 完整优化路线图（15 项已完成 + 1 项待做）
├── CHANGELOG.md                        # 详细更新日志（Phase 1-10）
├── PROJECT.md                          # 项目需求文档
├── UI_DESIGN.md                        # 界面视觉设计稿
├── README.md                           # GitHub 介绍
└── HANDOFF.md                          # 本文档
```

**关键文件行数**：app.rs ~1500 | draw.rs ~700 | api/mod.rs ~300 | api/types.rs ~150 | file_ops.rs ~164 | session.rs ~115 | config.rs ~100

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
- `StreamResult::Done { input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens }` — 生成完成，含用量和缓存统计
- `StreamResult::Error(err)` — API 错误，UI 显示红色错误信息

---

## 5. API 对接详情

### 认证（双模式）

**模式 1：Anthropic Token Plan（默认）**
- Base URL: `https://token-plan-sgp.xiaomimimo.com/anthropic`
- Header: `api-key: tp-{your_key}` + `anthropic-version: 2023-06-01`

**模式 2：Bearer（普通 MiMo API）**
- Base URL: `https://api.xiaomimimo.com/v1`
- Header: `Authorization: Bearer {your_key}`

配置项 `auth_type`: `"anthropic"` 或 `"bearer"`。

### 流式事件类型
```json
// message_start — 首个事件，含 input_tokens + cache 信息
{"type": "message_start", "message": {"usage": {"input_tokens": 123, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 456}}}

// content_block_delta — 逐 token
{"type": "content_block_delta", "delta": {"type": "text_delta", "text": "你好"}}

// message_delta — 最终事件，含 output_tokens
{"type": "message_delta", "usage": {"output_tokens": 789}}

// message_stop — 结束
{"type": "message_stop"}
```

### 可用模型
| 模型 | 说明 |
|------|------|
| mimo-v2-flash | 极速轻量，日常编码（默认） |
| mimo-v2.5 | 全模态 |
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
  "model": "mimo-v2-flash",
  "auth_type": "anthropic",
  "skills": {
    "lint": "cargo clippy 2>&1",
    "test": "cargo test 2>&1",
    "build": "cargo build 2>&1",
    "git": "git log --oneline -10",
    "diff": "git diff"
  }
}
```

会话存储于：`~/.config/mimo-opt/sessions/{timestamp}.json`
文件备份于：`.mimo-opt/backups/{filename}.{unix_ts}.bak`

---

## 7. 界面布局

```
╭─ MiMo-OPT [session-736_1430] ────── ⊛ API OK ──────── ⚡ mimo-v2-flash ─╮
╰──────────────────────────────────────────────────────────────────────────╯

   你好                                                          ← 用户消息（亮白）
                                                                 ← 空行分隔
   你好！有什么可以帮你的？                                       ← MiMo 回复（稍暗灰）

   ┌─ rust ──────────────────────────────────────────────────┐  ← 代码块（语法高亮）
   │  fn main() {                                             │
   │      println!("hello");                                  │
   │  }                                                       │
   └──────────────────────────────────────────────────────────┘

   ⠏ thinking...   ▎                                          ← 流式 spinner + 光标

─────────────────────────────────────────────────────────────────────────────

  /lint  /list  /help                                          ← 技能提示浮层（输入 / 时）

   › 输入消息...▎                                  Ctrl+Enter ↵  ← 输入框

╭───────────────────────────────────────────────────────────────────────────╮
│  ◉ 3.2K tok  ↓123 ↑456  ♻ 78%  ￥0.0004      [↑3]                    │  ← 状态栏
╰───────────────────────────────────────────────────────────────────────────╯
```

### 确认弹窗（居中覆盖）
```
╭────────────────────────────────────────────────╮
│  ⚠ 确认写入文件？                               │
│  src/main.rs (1234 B)                           │
│  ────────────────────────                       │
│  y 确认    n 取消    d 详情                      │
╰────────────────────────────────────────────────╯
```

### 搜索栏（Ctrl+F 时替代状态栏位置）
```
  / hello world  2/5                            Enter ↵  Esc ✕
```

### 配色（Tokyo Night）
- 标题紫: `#7aa2f7` | 模型青: `#7dcfff` | 用户白: `#c0caf5` | MiMo灰: `#a9b1d6`
- 状态栏数据: `#7dcfff` | 成功绿: `#9ece6a` | 错误红: `#f7768e` | 边框灰: `#565f89`
- 代码块背景: `#1a1b26` | 关键字紫: `#bb9af7` | 字符串绿: `#9ece6a` | 函数蓝: `#7aa2f7`

---

## 8. 已实现功能（全部完成）

### 核心
- [x] MiMo Token Plan API 流式对话（SSE）+ 双 API 认证（anthropic / bearer）
- [x] ratatui 终端界面（Tokyo Night 配色）
- [x] 流式逐 token 输出 + spinner
- [x] UTF-8 跨 chunk 安全解码
- [x] API 状态探测 + 错误传递到 UI
- [x] token 用量追踪 + 费用计算（按 input/output 分开）

### 缓存优化（P0-0）
- [x] 渐进式多断点：messages[0] + 每 6 条追加一个断点
- [x] System prompt 去日期化（日期移入首条 user message）
- [x] 命中率公式修正：cache_read / (input_tokens + cache_read)
- [x] 状态栏实时显示 `♻ XX%` 缓存命中率

### 代码块（P0-1）
- [x] syntect 语法高亮（自定义 Tokyo Night 22 种 scope 配色）
- [x] 语言标签显示（`┌─ rust ──`）
- [x] 流式 buffer 中未闭合代码块同样高亮

### 交互（P0-2 + P1-0/1）
- [x] 输入历史浏览（↑/↓ 切换，自动保存草稿）
- [x] 聊天区滚动（PageUp/Down 翻页，Ctrl+Home 回底，状态栏 [↑N] 指示器）
- [x] 消息搜索（Ctrl+F，Enter/Shift+Enter 跳转，黄色高亮）

### 技能系统（P1-5）
- [x] 参数传递 `{args}` 占位符
- [x] 5 个内置默认技能（lint/test/build/git/diff）
- [x] 输出 2000 字截断 + 耗时显示
- [x] 输入 `/` 时浮动提示 + `/help` 列出全部
- [x] 技能管理：`/skills` `/addskill` `/rmskill`，自动保存 config.json

### 文件操作（P1-3）
- [x] `/read <path>[:start[-end]]` — 读取文件注入上下文（带行号）
- [x] `/write <path>` — 提取最后代码块写入文件
- [x] `/edit <path> <old> <new>` — 精确字符串替换
- [x] 路径沙箱（禁止 `../` 穿越 + 绝对路径 + `.git/` 写入）
- [x] 自动备份到 `.mimo-opt/backups/`（保留 5 个版本）
- [x] Windows UNC 路径兼容

### 确认机制（P1-4）
- [x] 居中红色弹窗（y 确认 / n 取消 / d 详情）
- [x] 文件写入/编辑/清空对话/退出 需确认
- [x] 确认状态下屏蔽其他输入

### 其他
- [x] Content enum 改造（`#[serde(untagged)]`，为 Image/ToolUse 预留）
- [x] 代码块复制（Ctrl+Y，复制最后一个代码块到剪贴板）
- [x] 多会话持久化（自动加载最近会话，Ctrl+N 新建，F2 切换，每 5 条消息自动保存）
- [x] URL 预计算 + reqwest 连接池配置
- [x] 启动性能优化（theme OnceLock 缓存）
- [x] 终端清理保护（scopeguard）
- [x] 项目文件自动扫描注入 system prompt（100 个文件）

---

## 9. 快捷键速查

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Enter` | 发送消息 / 执行技能 |
| `Ctrl+C` / `Esc` | 中断生成（不退出） |
| `Ctrl+Q` | 退出（有对话时弹确认框） |
| `Ctrl+N` | 新建会话 |
| `F2` | 切换会话 |
| `Ctrl+F` | 搜索消息 |
| `Ctrl+Y` | 复制最后一个代码块到剪贴板 |
| `↑/↓` | 浏览输入历史 |
| `PageUp/PageDown` | 翻页浏览对话 |
| `Ctrl+Home` | 回到对话底部 |
| `Home/End` | 光标行首/行尾 |
| `Ctrl+A/Ctrl+E` | 光标行首/行尾（Emacs 风格） |

---

## 10. 内置命令速查

| 命令 | 功能 | 需确认 |
|------|------|--------|
| `/read <path>[:range]` | 读取文件注入上下文 | 否 |
| `/write <path>` | 提取代码块写入文件 | 是（文件已存在时） |
| `/edit <path> <old> <new>` | 精确字符串替换 | 是 |
| `/clear` | 清空当前对话 | 是 |
| `/skills` | 列出全部技能 | 否 |
| `/addskill <name> <cmd>` | 添加技能 | 否 |
| `/rmskill <name>` | 删除技能 | 否 |
| `/help` | 显示帮助 | 否 |
| `/skill_name [args]` | 执行技能命令 | 否 |

---

## 11. 关键实现细节

### 缓存策略
```rust
fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    // messages[0] 始终设断点（system prompt + 首条消息锚点）
    messages[0].cache_control = Some(CacheControl { cache_type: "ephemeral".to_string() });
    // 每 6 条消息追加一个断点（3 轮对话一层）
    for i in (3..n).step_by(6) {
        messages[i].cache_control = Some(CacheControl { cache_type: "ephemeral".to_string() });
    }
}
```
- System prompt 不含日期，跨天存活
- 日期注入到首条 user message（`messages.len() == 1` 时）
- 预期命中率：5 轮对话后稳定 70%+

### Content enum
```rust
#[serde(untagged)]
pub enum Content { Text(String) }
// 辅助：text(s), as_str(), as_mut_str()
// 未来扩展：Image, ToolUse, ToolResult
```

### 文件操作安全
- `validate_path()`：禁止 `../` 穿越、绝对路径、`.git/` 写入
- `strip_unc_prefix()`：去除 Windows `\\?\` 前缀
- `backup_file()`：写入前备份到 `.mimo-opt/backups/`，保留 5 个版本

### 会话持久化
- 存储：`~/.config/mimo-opt/sessions/{unix_timestamp}.json`
- 自动保存：每 5 条消息 + 退出时
- 手动操作：Ctrl+N 新建、F2 切换

### 异步任务调度
```rust
let client = Arc::clone(&state.client);  // Arc 共享
let tx = token_tx.clone();               // channel sender
tokio::spawn(async move {
    let result = client.send_message_stream(&messages, tx.clone()).await;
    if let Err(e) = result {
        let _ = tx.send(StreamResult::Error(e.to_string()));
    }
});
```

---

## 12. 快速开始

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

## 13. 待完成方向

- [ ] **P2-8** 桌面端迁移（Tauri）— 已决定终端版先交付，桌面端后续再做
- [ ] 导出会话为 Markdown
- [ ] 模型热切换（/model 命令）
- [ ] MCP 支持
- [ ] HTTP API 模式（无头，CI/CD 集成）

---

## 14. 参考项目

| 项目 | 说明 | 可借鉴 |
|------|------|--------|
| DeepSeek-Reasonix | 前缀缓存优先循环 | 缓存分层架构、成本控制 |
| DeepSeek-TUI | Rust+ratatui 终端 Agent | 工具系统、交互模式、LSP |
| MiMo Code | 基于 Claude Code 改造 | TypeScript+Ink 架构参考 |
| MiMo-Skills | 官方技能包 | MCP/技能系统参考 |
