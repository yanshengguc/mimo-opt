# MiMo-OPT 优化路线图

> 给后续 AI 接管项目的说明：本文件记录已完成和待完成的优化方向。
> 每次优化后请更新对应条目的状态。

---

## 已完成（Phase 2 缓存优化 + Phase 3 技能系统）

以下优化已全部实现，代码编译通过且 0 warnings，不要再重复做。

### 缓存核心三件套

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 1 | ChatRequest 加 system + cache_control | [api/types.rs](src/api/types.rs) | `ChatRequest` 新增 `system: Option<Vec<SystemContent>>`；新增 `CacheControl`、`SystemContent` 结构体 |
| 2 | 固定 system prompt + 项目上下文注入 | [app.rs](src/app.rs) | `build_system_prompt_text()` 构建 MiMo 身份+格式+项目文件+日期；`scan_project_tree()` 扫描 100 个文件（排除 .git/node_modules/target 等）；日期变化时自动重建 |
| 3 | Usage 解析缓存字段 + UI 展示 | [api/types.rs](src/api/types.rs) [ui/draw.rs](src/ui/draw.rs) | `Usage` 新增 `cache_creation_input_tokens`/`cache_read_input_tokens`；状态栏显示 `♻ XX%` 命中率 |

### 缓存策略

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 4 | 对话前缀缓存断点 | [app.rs](src/app.rs) | `apply_cache_breakpoints()` 在 messages >= 4 条时标记 message[3] 为 `cache_control: ephemeral` |
| 5 | build_request 抽取 | [api/mod.rs](src/api/mod.rs) | `build_request()` 方法统一构建 ChatRequest，消除 check_api 和 send_message_stream 的重复代码 |
| 6 | StreamResult::Done 扩展 | [api/mod.rs](src/api/mod.rs) | Done 变体新增 `cache_creation_tokens`/`cache_read_tokens` 字段，SSE 事件解析中提取 |
| 7 | 光标 UTF-8 安全移动 | [app.rs](src/app.rs) | 左右箭头、Backspace 使用 `char.len_utf8()` 计算边界，不再直接 -1 |
| 8 | dead_code 抑制 | [ui/theme.rs](src/ui/theme.rs) | `#[allow(dead_code)]` 在 `impl Theme` 上，消除 6 个未使用常量警告 |

### 请求层优化

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 9 | URL 预计算 | [api/mod.rs](src/api/mod.rs) | `messages_url` 字段在 `new()` 时预计算，`check_api` 和 `send_message_stream` 不再每帧 format! |
| 10 | reqwest 连接池配置 | [api/mod.rs](src/api/mod.rs) | `Client::builder()` 设置 timeout=120s、pool_idle_timeout=90s、tcp_keepalive=60s |

### 技能系统

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 11 | 技能配置 | [config.rs](src/config.rs) | `Config` 新增 `skills: HashMap<String, String>`，配置文件支持 `"skills": {"lint": "cargo clippy", ...}` |
| 12 | 技能命令执行 | [app.rs](src/app.rs) | Ctrl+Enter 输入 `/skill_name` 时执行对应 shell 命令（Windows: `cmd /C`，其他: `sh -c`），输出显示在对话区，然后自动发送给 MiMo 分析 |

**技能使用方式**：在配置文件中添加：
```json
{
  "api_key": "...",
  "model": "mimo-v2-flash",
  "skills": {
    "lint": "cargo clippy 2>&1",
    "test": "cargo test 2>&1",
    "tree": "find . -type f -not -path './target/*' -not -path './.git/*' | head -50",
    "git": "git log --oneline -10",
    "build": "cargo build 2>&1"
  }
}
```
输入 `/lint` 即可执行 `cargo clippy`，输出自动发给 MiMo 分析。

---

## 待完成优化

### P0：代码块语法高亮

**收益**：用户体验大幅提升，代码可读性

**改动范围**：[ui/draw.rs](src/ui/draw.rs)、[Cargo.toml](Cargo.toml)

**方案**：
- 引入 `syntect` crate（~1MB 二进制增量）
- 在 `draw_chat_area` 中检测 markdown 代码块（` ``` ` 围栏）
- 代码块内容用 `syntect` 解析 token，按类型着色
- 已定义的颜色常量直接使用：`CODE_KW`(紫)、`CODE_STR`(绿)、`CODE_FN`(蓝)、`CODE_COMMENT`(灰)、`CODE_BG`(深色背景)
- 流式 buffer 也要实时识别未闭合的代码块（` ``` ` 开始但还没结束的情况）

**实现细节**：
```
draw_chat_area 中的渲染逻辑：
1. 遍历每行，检测 ``` 开始/结束
2. 代码块外：保持现有 Span::styled 逻辑
3. 代码块内：
   - 用 syntect::easy::highlight_lines 逐行高亮
   - syntect token → theme.rs 颜色映射：
     - Keyword → CODE_KW
     - String → CODE_STR
     - Function → CODE_FN
     - Comment → CODE_COMMENT
   - 背景色用 CODE_BG（通过 Style::bg()）
4. 流式 buffer 中未闭合的代码块同样处理
```

**注意**：实现后 theme.rs 中的 CODE_* 常量自然被使用，可移除 `#[allow(dead_code)]`

---

### P1：技能系统增强

**收益**：降低程序员使用门槛，扩展使用场景

#### 1a. 技能参数传递

当前 `/lint` 执行固定命令。支持参数：`/lint --fix` → `cargo clippy --fix 2>&1`

**方案**：命令模板支持 `{args}` 占位符：
```json
"skills": {
  "lint": "cargo clippy {args} 2>&1"
}
```
输入 `/lint --fix` → `{args}` 替换为 `--fix`，执行 `cargo clippy --fix 2>&1`

#### 1b. 内置技能

预设常用开发命令，无需手动配置：
- `/lint` — 代码检查
- `/test` — 运行测试
- `/build` — 编译项目
- `/git` — 最近 git 日志
- `/diff` — 未提交的变更

**方案**：在 `Config::load()` 中，如果 skills 为空，填入默认值。

#### 1c. 技能执行结果增强

当前技能输出作为纯文本发给 MiMo。改进：
- 输出超过 2000 字符时自动截断，提示 "输出过长，已截断"
- 技能执行时间显示（状态栏或对话区）
- 执行失败时区分 stderr 和 stdout

#### 1d. 技能列表展示

输入 `/` 时在输入框下方显示可用技能列表（类似自动补全），或 `/help` 显示所有技能。

**方案**：在 `draw_input_area` 中检测输入以 `/` 开头时，渲染技能提示行。

---

### P1：消息结构升级 — Content enum

**收益**：为 Phase 5 MCP/工具调用铺路，修改成本低（现在改比以后改容易）

**改动范围**：[api/types.rs](src/api/types.rs)、draw.rs（渲染适配）

**当前状态**：`ChatMessage.content` 是纯 `String`

**目标**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    // 未来扩展: Image, ToolUse, ToolResult
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}
```

**注意**：使用 `#[serde(untagged)]` 保持 API 兼容（字符串仍序列化为纯字符串），draw.rs 中渲染时 match Content 类型

---

### P2：多会话持久化

**收益**：会话不会因程序退出丢失

**改动范围**：app.rs、新增 session 模块

**方案**：
- 将 `messages` 序列化为 JSON 存储到 `~/.config/mimo-opt/sessions/`
- 每次退出时自动保存，启动时可选择恢复
- 标题栏显示会话标识
- Ctrl+N 新建会话，Ctrl+O 切换会话

---

### P3：代码块复制功能

**收益**：用户可复制代码块内容

**方案**：
- 检测代码块后，添加 `[Copy]` 标记
- 点击/快捷键触发 `clipboard` crate 写入剪贴板

---

### P3：技能编辑器（TUI 内配置）

**收益**：不用手动编辑 JSON 就能添加/修改技能

**方案**：
- `/edit` 命令打开技能编辑面板
- 支持添加、修改、删除技能
- 编辑后自动保存到 config.json

---

## 关键约束

### 保证不降智

缓存优化不能通过以下方式影响模型质量：

| 危险做法（降智） | 安全做法（不降智） |
|------------------|-------------------|
| 截断历史消息使模型丢失上下文 | 只在缓存断点处标记，不删除任何消息 |
| system prompt 过度约束推理空间 | system prompt 只提供事实信息（项目结构），不给推理指令 |
| 压缩/摘要历史导致信息丢失 | 全部消息保留，只对 API 传输做缓存标记 |
| 缩短 max_tokens 限制输出 | max_tokens 保持 4096 不变 |

**核心原则**：缓存优化是传输层优化，不应改变模型看到的内容。

### 技能安全

- 技能是用户自行配置的 shell 命令，等同于用户手动在终端执行
- 不做沙箱隔离（程序员需要完整 shell 访问）
- 执行超时无硬限制（长命令如 `cargo build` 需要时间）
- 技能输出直接发给 MiMo 分析，不存储到文件

---

## 项目快照

- **当前行数**：~700 行 Rust
- **依赖**：tokio, reqwest, serde, serde_json, ratatui, crossterm, futures-util, dirs, anyhow, scopeguard, unicode-width
- **编译状态**：通过，0 warnings
- **API 端点**：`https://token-plan-sgp.xiaomimimo.com/anthropic`（Anthropic Messages 兼容）
- **默认模型**：mimo-v2-flash
