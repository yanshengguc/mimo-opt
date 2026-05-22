# MiMo-OPT 项目交接文档

> 给接手的 AI：读完这份文档就能理解项目全貌，直接开始优化。

---

## 1. 项目是什么

小米 MiMo Token Plan API 的终端聊天工具。目标用户是程序员。

**一句话：** 类似 ChatGPT/Claude 的终端版，但接的是小米 MiMo API，Tokyo Night 配色，简洁不简陋。

**当前状态：** v0.5.2，多 Provider 多模型支持 + 缓存 v3(85-92%命中率) + 联网搜索 + 15 项 UX 优化 + 撤回/费用预估/代理/Logo/通知/diff预览 + clippy 0 warnings。

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
| 剪贴板 | arboard 3 | 复制代码块 (X11/Wayland/Windows/macOS) |
| 日志 | log 0.4 + env_logger 0.11 | 结构化日志（RUST_LOG=mimo_opt=debug） |
| 其他 | scopeguard, unicode-width, futures-util, dirs, anyhow | |

编译命令：`cd C:/test/mimo-opt && cargo build`
Rust 路径：`~/.cargo/bin/`（可能不在 PATH 中，需 `export PATH="$PATH:~/.cargo/bin"`）

---

## 3. 文件结构

```
C:/test/mimo-opt/
├── Cargo.toml                          # 依赖配置 (v0.5.2)
├── src/
│   ├── main.rs                         # 入口：加载配置，检查 api_key，启动 app
│   ├── app.rs                          # 核心：AppState + 事件循环 + 键盘处理 + 异步任务调度（~780 行）
│   ├── commands.rs                     # 命令系统：/model /provider /read /write /edit + 技能执行 + 流式请求
│   ├── config.rs                       # 配置：ProviderPreset + ModelInfo + PROVIDERS(3xProvider=9xModel) + 定价回退
│   ├── prompt.rs                       # System prompt：构建/缓存断点/日期注入/自适应布局
│   ├── search.rs                       # 联网搜索：DDG HTML 解析 + 结果格式化
│   ├── scanner.rs                      # 项目文件树扫描
│   ├── file_ops.rs                     # 文件操作：read/write/edit，路径沙箱 + 自动备份
│   ├── session.rs                      # 会话持久化：Session 结构 + JSON 存储
│   ├── util.rs                         # 工具：时间戳/工作目录/token估算/费用预估
│   ├── api/
│   │   ├── mod.rs                      # MiMoClient：send_message_stream() + 重试 + 余额 + 联网搜索
│   │   └── types.rs                    # 类型：Content enum, Anthropic/OpenAI 请求/响应/流式
│   └── ui/
│       ├── mod.rs                      # UI 模块入口
│       ├── theme.rs                    # 3 套主题（tokyo-night/nord/catppuccin）热切换
│       ├── logo.rs                     # 启动 Logo：芒果猫 8×5 ANSI 色块
│       └── draw.rs                     # 界面绘制：标题栏/对话区/输入框(自适应)/状态栏/确认弹窗/搜索栏
├── OPTIMIZATION.md                     # 完整优化路线图（16/23 完成）
├── CHANGELOG.md                        # 详细更新日志（v0.1.0 → v0.5.2）
├── PROJECT.md                          # 项目需求文档
├── UI_DESIGN.md                        # 界面视觉设计稿
├── README.md                           # GitHub 介绍
└── HANDOFF.md                          # 本文档
```

**关键文件行数**：app.rs ~780 | commands.rs ~1070 | draw.rs ~880 | api/mod.rs ~640 | config.rs ~300

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

### 多 Provider / 多模型

模型定义在 `config.rs` 的 `PROVIDERS` 常量中，每个 Provider 持有 `models: &[ModelInfo]`，每个模型含 `name` + `desc` + 独立定价。用户通过 `/model` 命令浏览和切换。

**当前模型清单（v0.5.2）**：

| Provider | 模型 | 描述 | 输入价 ¥/Mtok | 输出价 ¥/Mtok |
|----------|------|------|----------------|----------------|
| MiMo | mimo-v2-flash | 轻量快速，日常对话 | 2.0 | 8.0 |
| MiMo | mimo-v2-pro | 专业推理，复杂任务 | 6.0 | 24.0 |
| DeepSeek | deepseek-chat | V3 标准对话，性价比高 | 1.0 | 2.0 |
| DeepSeek | deepseek-reasoner | R1 深度推理，数学/代码/逻辑 | 4.0 | 16.0 |
| DeepSeek | deepseek-v4-flash | V4 轻量快速，日常高频 | 1.5 | 3.0 |
| DeepSeek | deepseek-v4-pro | V4 旗舰，全能最强 | 6.0 | 24.0 |
| OpenAI | gpt-4o-mini | 轻量快速，日常使用 | 1.25 | 5.0 |
| OpenAI | gpt-4o | 全能旗舰，多模态 | 2.5 | 10.0 |
| OpenAI | gpt-4-turbo | 高性能推理 | 10.0 | 30.0 |

**定价回退链**：模型精确匹配 → Provider 默认模型 → 硬编码兜底(2.0/8.0)

### 添加新模型

在 `PROVIDERS` 对应 Provider 的 `models` 数组中添加 `ModelInfo` 条目即可。`/model` 命令自动列出，`/model <name>` 自动有补全提示。无需改其他文件。

---

## 6. 配置文件

自动创建于：`~/.config/mimo-opt/config.json`（Windows: `%APPDATA%\mimo-opt\config.json`）

```json
{
  "provider": "deepseek",
  "api_key": "sk-your-key",
  "base_url": "https://api.deepseek.com",
  "model": "deepseek-chat",
  "auth_type": "bearer",
  "api_format": "openai",
  "max_tokens": 4096,
  "theme": "tokyo-night",
  "proxy_url": null,
  "temperature": null,
  "top_p": null,
  "web_search": {
    "enabled": true,
    "engine": "ddg",
    "max_results": 5,
    "timeout_secs": 10
  },
  "skills": {
    "lint": "cargo clippy 2>&1",
    "test": "cargo test 2>&1",
    "build": "cargo build 2>&1",
    "git": "git log --oneline -10",
    "diff": "git diff"
  }
}
```

**字段说明**：
- `provider`: 必填，"mimo"|"deepseek"|"openai"|"custom"。决定 Provider 预设（base_url/model/auth/定价默认值）
- `model`: 可选，覆盖 Provider 默认模型。`/model` 列出当前 Provider 已知模型
- `proxy_url`: 可选，HTTP/SOCKS5 代理（如 `"http://127.0.0.1:7890"`）
- `temperature`/`top_p`: 可选，推理参数，不设则用 API 默认值
- `web_search`: 联网搜索配置，`/search` 命令使用

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
- [x] Ctrl+V 粘贴剪贴板内容到输入框（v0.3.4）
- [x] 代码块深色背景 #1a1b26（v0.3.4）
- [x] 会话 token/费用持久化，重启/切换后恢复（v0.3.4）
- [x] 搜索高亮对比度提升（v0.3.4）
- [x] clippy 零警告（v0.3.4，27 → 0）
- [x] 长消息发送确认（>5 行弹确认，取消恢复输入）（v0.3.5）
- [x] 对话轮次分隔线（虚线分隔不同问答轮次）（v0.3.5）
- [x] 终端最小尺寸检查（<60×20 居中红色警告）（v0.3.5）
- [x] 会话导出 Markdown — `/export [path]`（v0.3.5）
- [x] 错误历史 — API 错误自动记录 20 条 + `/errors` 查看（v0.3.5）
- [x] 启动省配额 — 移除 `check_api()` 探测（v0.3.5）
- [x] 结构化日志 — `log` + `env_logger`，17 个点位覆盖关键路径（v0.3.7）
- [x] API 自动重试 — 5xx/429/网络错误 3 次指数退避重试（v0.3.7）
- [x] DeepSeek 余额查询 — 启动时/切换 provider 时查询，标题栏展示（v0.3.7）
- [x] 修复 output_tokens 流式计数的粗略估算（v0.3.7）
- [x] 余额货币单位 + 颜色预警（<¥1 红/<¥5 黄/≥¥5 绿）（v0.3.8）
- [x] 会话名友好化 — `{目录名}_{HHMM}` 替代 epoch 天数（v0.3.8）
- [x] 退出用量汇总 — 终端打印消息数/token/命中率/费用（v0.3.8）
- [x] 首次启动 API 状态引导 + 余额查询失败可见（v0.3.8）
- [x] 引用块视觉 — `>` 竖线+缩进+斜体（v0.3.9）
- [x] 内联 Markdown — 粗体/斜体/行内代码（v0.3.9）
- [x] Markdown 渲染完善 — 无序/有序列表 + `[链接](url)` + `---` 分隔线（v0.4.0）
- [x] 主题热切换 — 3 套内置主题 + `/theme` 命令（v0.4.0）
- [x] 缓存 v3 — 日期前置 + System Prompt 拆分 + 自适应断点，命中率 85-92%（v0.5.0）
- [x] HTTP/SOCKS5 代理 — `proxy_url` 配置项（v0.5.0）
- [x] 联网搜索 — `/search` 命令，DDG + DeepSeek 双引擎（v0.5.0）
- [x] 发送前费用预估 — 输入框右侧实时显示 `~¥ tok`（v0.5.0）
- [x] Ctrl+Z 撤回 — 移除最后对话轮次，恢复输入（v0.5.0）
- [x] 输入框自适应扩展 — 3~半屏动态高度（v0.5.0）
- [x] syntect 异步加载 — 首屏不阻塞（v0.5.0）
- [x] /edit diff 预览 — 确认弹窗红删绿增（v0.5.0）
- [x] 回复完成通知 — 终端响铃 `\x07`（v0.5.0）
- [x] temperature / top_p 可配置（v0.5.0）
- [x] cli-clipboard → arboard，Wayland 原生支持（v0.5.0）
- [x] 芒果猫启动 Logo — 8×5 ANSI 色块 + 版本/Provider/余额（v0.5.0）
- [x] 代码去重 — draw.rs 内联解析合并为 `parse_inline_spans()`（v0.5.1）
- [x] DDG 搜索结果注入 AI 分析 — 不再仅展示原始结果（v0.5.1）
- [x] 多模型支持 — 每 Provider 多模型 + 独立定价 + /model 列出/切换（v0.5.2）
- [x] 模型中文描述 — `desc` 字段，`/model` 列出时一目了然（v0.5.2）
- [x] 二级补全提示 — `/model <partial>` + `/provider <partial>` 自动补全（v0.5.2）

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
| `Ctrl+Z` | 撤回最后一轮对话 |
| `Ctrl+V` | 粘贴剪贴板 |
| `↑/↓` | 浏览输入历史 |
| `PageUp/PageDown` | 翻页浏览对话 |
| `Ctrl+Home` | 回到对话底部 |
| `Home/End` | 光标行首/行尾 |
| `Ctrl+A/Ctrl+E` | 光标行首/行尾（Emacs 风格） |

---

## 10. 内置命令速查

| 命令 | 功能 | 需确认 |
|------|------|--------|
| `/model [name]` | 列出可用模型 / 切换模型 | 否 |
| `/provider <name>` | 切换 API 提供商 | 否 |
| `/theme [name]` | 列出主题 / 切换主题 | 否 |
| `/search <kw>` | 联网搜索（DDG/DeepSeek） | 否 |
| `/read <path>[:range]` | 读取文件注入上下文 | 否 |
| `/write <path>` | 提取代码块写入文件 | 是（文件已存在时） |
| `/edit <path> <old> <new>` | 精确字符串替换 | 是 |
| `/export [path]` | 导出会话为 Markdown | 否 |
| `/clear` | 清空当前对话 | 是 |
| `/errors` | 查看错误历史 | 否 |
| `/skills` | 列出全部技能 | 否 |
| `/addskill <name> <cmd>` | 添加技能 | 否 |
| `/rmskill <name>` | 删除技能 | 否 |
| `/help` | 显示帮助 | 否 |
| `/skill_name [args]` | 执行技能命令 | 否 |

---

## 11. 关键实现细节

### 缓存策略 (v3，prompt.rs)
- **W1 日期前置**: `inject_date_preamble()` 插入不含 cache_control 的日期消息，不污染 messages[0] 缓存
- **W2 System Prompt 拆分**: `build_system_content_split()` → stable block(缓存) + dynamic block(不缓存)
- **W3 自适应断点**: `apply_cache_breakpoints()` 按对话长度 4 档分配断点（1-3/4-8/9-16/17+），优先覆盖最近一轮
- 预期命中率：85-92% (vs v2 的 70-85%)

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
export PATH="$PATH:~/.cargo/bin"

# 编译
cd C:/test/mimo-opt
cargo build

# 运行（首次会提示配置 api_key）
cargo run

# 编辑配置
notepad "%APPDATA%\mimo-opt\config.json"
```

---

## 13. 待完成方向

**P1 安全**:
- [ ] E2: 单元测试覆盖（优先 file_ops / config / prompt / cost / session）

**P2 体验**:
- [ ] U1: 会话侧边栏（Ctrl+B）
- [ ] C1: 消息列表 clone 优化（`Arc<Vec<ChatMessage>>`）
- [ ] C2: commands.rs / draw.rs 模块拆分

**P3 补充**:
- [ ] U9: Shell 管道集成（`echo "..." | mimo-opt --prompt`）
- [ ] N9: 快捷键可配置（keybindings.json）
- [ ] D1: 桌面端 Tauri 迁移

**待定**:
- [ ] GLM / 通义千问 / Kimi 等平台预设完善
- [ ] MCP 支持
- [x] U3: Markdown 渲染增强 — 粗体/斜体/行内代码 v0.3.9 + 列表/链接/分隔线 v0.4.0
- [x] U4: 主题热切换 — Tokyo Night/Nord/Catppuccin + /theme 命令 v0.4.0
- [x] U8: 引用块视觉支持（blockquote）— v0.3.9
- [ ] U9: Shell 管道集成

**v0.3.7 实测反馈（6 项，已全部修复 v0.3.8）**:
- [x] **F1**: 首次启动 `⊛ 发消息检测API` 引导
- [x] **F2**: 余额 `💰¥6.33` 含货币单位
- [x] **F3**: 余额 <¥1 红/<¥5 黄/≥¥5 绿
- [x] **F4**: 会话名 `{目录名}_{HHMM}`
- [x] **F5**: 余额查询失败 → error_message
- [x] **F6**: 退出终端打印用量汇总

**已决定后续再做**:
- [ ] 桌面端迁移（Tauri）— 终端版先交付
- [ ] MCP 支持

---

## 14. 参考项目

| 项目 | 说明 | 可借鉴 |
|------|------|--------|
| DeepSeek-Reasonix | 前缀缓存优先循环 | 缓存分层架构、成本控制 |
| DeepSeek-TUI | Rust+ratatui 终端 Agent | 工具系统、交互模式、LSP |
| MiMo Code | 基于 Claude Code 改造 | TypeScript+Ink 架构参考 |
| MiMo-Skills | 官方技能包 | MCP/技能系统参考 |
