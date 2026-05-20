# MiMo-OPT 优化路线图

> 给后续 AI 接管项目的说明：本文件记录已完成和待完成的优化方向。
> 每次优化后请更新对应条目的状态。

---

## 已完成（Phase 2-11 + v0.3.0-v0.3.6）

以下优化已全部实现，代码编译通过且 0 errors，不要再重复做。

### v0.3.x：Provider 预设 + DeepSeek 集成 + 实测修复

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| D1 | Provider 预设系统 | [config.rs](src/config.rs) | `provider` 字段 + `ProviderPreset` 结构 + `PROVIDERS` 常量表（mimo/deepseek/openai），含各平台费率 |
| D2 | OpenAI stream_options | [api/types.rs](src/api/types.rs) | `OpenAIRequest` 新增 `stream_options.include_usage`，DeepSeek 流式返回真实 token 用量 |
| D3 | Usage serde alias | [api/types.rs](src/api/types.rs) | `Usage` 字段加 `#[serde(alias)]`：同时兼容 Anthropic 的 `input_tokens`/`output_tokens` 和 OpenAI 的 `prompt_tokens`/`completion_tokens` |
| D4 | OpenAI SSE 用法捕获 | [api/mod.rs](src/api/mod.rs) | `stream_openai()` 从 SSE 事件中捕获 `usage` 字段 |
| D5 | System prompt 传 OpenAI | [api/mod.rs](src/api/mod.rs) | `build_openai_request` 接收 system 参数，OpenAI 格式不再丢弃系统提示词 |
| D6 | SSE partial 缓冲 | [api/mod.rs](src/api/mod.rs) | 两套 SSE 解析器均新增 `partial` 缓冲区，跨 chunk 分割事件不再丢失 |
| D7 | Provider 感知 UI | [ui/draw.rs](src/ui/draw.rs) [app.rs](src/app.rs) | 标题栏动态显示 `{Provider}-OPT`；系统提示词按 provider 输出不同 AI 身份 |
| D8 | Provider 感知计费 | [app.rs](src/app.rs) [config.rs](src/config.rs) | `calculate_cost` 改为调用 `config.input_price()` / `config.output_price()` |
| D9 | 启动引导 5 方案 | [main.rs](src/main.rs) | 首次运行提示含 MiMo/DeepSeek/OpenAI/MiMo API/自定义，各方案含完整 JSON 示例 |

### v0.3.2：P0/P1 安全与体验收官

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| S3 | 技能命令注入防护 | [app.rs](src/app.rs) | `shell_escape()` 函数：Unix 单引号包裹转义，Windows `^` 转义 `%^&<>|"()!` |
| M1 | `/model` 命令 | [app.rs](src/app.rs) [api/mod.rs](src/api/mod.rs) | 运行时切换模型不中断会话，自动保存配置 |
| M2 | `/provider` 命令 | [app.rs](src/app.rs) [api/mod.rs](src/api/mod.rs) | 运行时切换 provider 预设，自动更新 endpoint/auth/费率，重建 system prompt |
| M3 | 长对话自动管理 | [app.rs](src/app.rs) [ui/draw.rs](src/ui/draw.rs) | `MAX_MESSAGES=200` 超限截断保留 150，160 条警告，状态栏 `✉ N` 计数 |
| M4 | spawn 取消机制 | [app.rs](src/app.rs) | `tokio::select!` + `oneshot` channel，3 个 spawn 点全接入，Ctrl+C/Esc 真正中止 HTTP 请求 |
| — | ClientSettings RwLock | [api/mod.rs](src/api/mod.rs) | 可变字段收敛到 `RwLock<ClientSettings>`，block scope 确保 guard 在 `.await` 前释放 |

### 已支持的 API

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 1 | ChatRequest 加 system + cache_control | [api/types.rs](src/api/types.rs) | `ChatRequest` 新增 `system: Option<Vec<SystemContent>>`；新增 `CacheControl`、`SystemContent` 结构体 |
| 2 | 固定 system prompt + 项目上下文注入 | [app.rs](src/app.rs) | `build_system_prompt_text()` 构建 MiMo 身份+格式+项目文件+日期；`scan_project_tree()` 扫描 100 个文件（排除 .git/node_modules/target 等）；日期变化时自动重建 |
| 3 | Usage 解析缓存字段 + UI 展示 | [api/types.rs](src/api/types.rs) [ui/draw.rs](src/ui/draw.rs) | `Usage` 新增 `cache_creation_input_tokens`/`cache_read_input_tokens`；状态栏显示 `♻ XX%` 命中率 |

### 缓存策略

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| 4 | 对话前缀缓存断点 v1 | [app.rs](src/app.rs) | `apply_cache_breakpoints()` 在 messages >= 4 条时标记 message[3] 为 `cache_control: ephemeral`（v2 已升级为渐进式多断点） |
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

### v0.3.6：代码去重 + 架构优化

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| R1 | SSE 流式解析提取 | [api/mod.rs](src/api/mod.rs) | `process_sse_stream()` 泛型函数 + `SseAction` enum，消除 Anthropic/OpenAI 两套流解析器的 UTF-8 解码 + 事件分割重复代码 |
| R2 | 代码块提取统一 | [app.rs](src/app.rs) | `collect_code_blocks()` 改用 `extract_code_blocks_from_text()`，移除第 4 套内联代码块解析 |
| R3 | 日期注入 + 异步调度提取 | [app.rs](src/app.rs) | `inject_date_if_needed()` + `spawn_stream_request()`，3 处重复的日期注入+clone+断点+spawn 统一 |
| R4 | 内置命令常量 | [app.rs](src/app.rs) | `BUILTIN_COMMANDS` 常量替代硬编码列表，消除 `/help` 与提示不一致风险 |

---

## 待完成优化（v0.3.6 状态）

> 优先级定义：**P0**=安全/稳定 | **P1**=核心体验 | **P2**=重要但不急 | **P3**=锦上添花
> **P0 已全部修复（4/4 ✅），P1 已全部实现（4/4 ✅），v0.3.6 代码去重完成。以下 P2/P3 为剩余工作。**

### P0：安全修复 ✅ 全部完成

| # | 条目 | 文件 | 状态 |
|---|------|------|------|
| S1 | API Key 错误消息泄漏 | [api/mod.rs](src/api/mod.rs) | ✅ v0.3.1 |
| S2 | 配置文件权限未加固 | [config.rs](src/config.rs) [session.rs](src/session.rs) | ✅ v0.3.1 |
| S3 | 技能系统命令注入 | [app.rs](src/app.rs) | ✅ v0.3.2 |
| S4 | Config Debug 暴露密钥 | [config.rs](src/config.rs) | ✅ v0.3.1 |

### P1：核心体验 ✅ 全部完成

| # | 条目 | 文件 | 说明 |
|---|------|------|------|
| M1 | `/model` 命令 | [app.rs](src/app.rs) [api/mod.rs](src/api/mod.rs) | ✅ v0.3.2 |
| M2 | `/provider` 命令 | [app.rs](src/app.rs) [api/mod.rs](src/api/mod.rs) | ✅ v0.3.2 |
| M3 | 长对话自动管理 | [app.rs](src/app.rs) [ui/draw.rs](src/ui/draw.rs) | ✅ v0.3.2 |
| M4 | spawn 任务取消机制 | [app.rs](src/app.rs) | ✅ v0.3.2 |

### P2：工程化

| # | 条目 | 文件 | 说明 |
|---|------|------|------|
| E1 | API 自动重试 | [api/mod.rs](src/api/mod.rs) | 5xx/timeout/reset 最多 3 次指数退避重试 |
| E2 | 单元测试 | new `tests/` | file_ops / config / prompt / cost / session 优先 |
| E3 | SSE channel 有界化 | [app.rs](src/app.rs) [api/mod.rs](src/api/mod.rs) | ✅ 已修复 v0.3.1：`unbounded_channel` → `channel(256)` |
| E4 | DeepSeek 余额查询 | [main.rs](src/main.rs) | 启动时 `GET /user/balance` 显示余额，标题栏展示 |
| E5 | 结构化日志 | new dep `log`+`env_logger` | 关键路径加 info/debug/warn 日志 |
| E6 | app.rs 模块拆分 | [app.rs](src/app.rs) | ~1500 行拆为 app/prompt/scanner/cost/commands 多文件 |

### P3：锦上添花

| # | 条目 | 说明 |
|---|------|------|
| U1 | 会话侧边栏 (Ctrl+B) | UI_DESIGN 已规划，显示会话列表 |
| U2 | 代码块深色背景 | 当前只有竖线边框，无背景色 |
| U3 | Markdown 渲染增强 | 粗体/斜体/列表/引用/链接/分隔线 |
| U4 | 主题热切换 | 内置 Tokyo Night/Nord/Catppuccin，`/theme` 切换 |
| U5 | 终端最小尺寸警告 | 窗口 < 60×20 时显示警告 |
| U6 | 会话导出 Markdown | `/export [path]` 命令 |
| U7 | 对话轮次分隔线 | 虚线分隔不同轮次 |
| U8 | 引用块视觉支持 | `>` 引用用竖线+缩进渲染 |
| U9 | Shell 管道集成 | `echo "..." | mimo-opt --prompt` |
| U10 | check_api 省配额 | 跳过启动探测，首条消息自然检测 |
| D1 | 桌面端 Tauri 迁移 | core/gui 分层，feature flag 可选编译，一套代码双模式 |

---

### P0-0：缓存命中率 v2 —— 从 15-30% 提升到 70-85% ✅ 已完成

**目标**：修复当前缓存策略的三个致命缺陷，将命中率提升 50-60 个百分点。缓存优化是传输层优化，不改变模型看到的任何内容。

**前置知识——Anthropic prompt caching 机制**：
- 在 messages 数组的某个元素上设 `cache_control: { type: "ephemeral" }`，表示"此元素之前的所有内容可作为缓存前缀"
- 可在数组多处标记，API 服务端做 longest prefix match——匹配到最长有效前缀就从缓存读，后续部分正常计算
- `ephemeral` 类型：5 分钟 TTL，免费，不占用配额
- 关键约束：缓存的是**前缀**，前缀内容必须逐字节一致才能命中。这就是为什么 system prompt 里不能有日期——日期一变，整个前缀失效
- 本项目已通过 `build_system_content()` 给 system prompt 设了 `cache_control`，子任务 1 和 2 是在此基础上优化

**背景**：当前 `apply_cache_breakpoints` 只在 `messages[3]` 设一个静态断点。对话越长，未缓存的消息占比越大——10 轮对话后命中率仅 ~15%，与 README 宣称的 70-90% 差距巨大。

**三项改动（一次做完，严格不降智）**：

#### 子任务 1：渐进式多断点

**文件**：[app.rs](src/app.rs) — `apply_cache_breakpoints()`

**当前代码**（行 536-541）：
```rust
fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    if messages.len() >= 4 {
        messages[3].cache_control = Some(CacheControl {
            cache_type: "ephemeral".to_string(),
        });
    }
}
```

**改为**（v0.3.1 优化版：step_by(5) + 4 标记上限）：
```rust
fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 { return; }
    messages[0].cache_control = Some(CacheControl {
        cache_type: "ephemeral".to_string(),
    });
    let mut placed = 1;
    for i in (3..n).step_by(5) {
        if placed >= 4 { break; }  // Anthropic 最多 4 个标记
        messages[i].cache_control = Some(CacheControl {
            cache_type: "ephemeral".to_string(),
        });
        placed += 1;
    }
}
```

**原理**：Anthropic API 支持在 messages 数组中标记多个 `cache_control` 断点，服务端做 longest prefix match。多个断点形成层级，越长的对话命中越深的缓存层级。断点标记本身不改变消息内容。

**调用位置说明**：`apply_cache_breakpoints` 在两处被调用——普通消息发送路径（约行 344）和技能执行路径（约行 291）。两处都是 `state.messages.clone()` 之后、`send_message_stream()` 之前调用。新实现的函数签名不变，调用方无需修改。

**预期效果**（step_by(5) + 4 标记上限，20 条消息 / 10 轮对话场景）：

| 指标 | 改前 (step_by(6)) | 改后 (step_by(5)) |
|------|------------------|-------------------|
| 缓存覆盖的消息数 | 18 条 | 18 条 |
| 稳定后命中率 | ~75% | ~90% |
| 每轮节省 input tokens | ~2700 | ~3200 |
| 断点标记数（25条时） | 5 ⚠️ 超限 | 4 ✅ 合规 |

> 主要收益：间隔从 6 缩到 5 消除锯齿波动，4 标记上限防止超过 Anthropic API 限制

#### 子任务 2：System prompt 去日期化

**文件**：[app.rs](src/app.rs) — 涉及 4 个位置，按顺序改。

**问题**：`build_system_prompt_text()` 末尾拼入了日期，每天变化导致 system prompt 的 `cache_control` 标记失效，缓存每天需要重建一次（~500 tokens 白花）。

**方案**：system prompt 保持纯静态（身份 + 格式 + 项目文件），日期改为注入到首条 user message 的 content 中。模型仍然看到日期，但 system prompt 缓存不再每天失效。

**逐个位置修改**：

**位置 1**：`build_system_prompt_text()` 函数（约行 500-521）— 删除 date 参数和日期拼入逻辑。

当前签名和末尾：
```rust
fn build_system_prompt_text(project_tree: &str, date: &str) -> String {
    // ... 前面不变 ...
    text.push_str("\n\n## Current Date\n");
    text.push_str(date);
    text
}
```
改为：
```rust
fn build_system_prompt_text(project_tree: &str) -> String {
    // ... 前面不变，直接返回，不拼日期 ...
    text
}
```

**位置 2**：`AppState` 初始化（约行 90-91）— 去掉 date 参数。
```rust
// 改前
state.system_prompt_text = build_system_prompt_text(&state.project_tree, &today);
// 改后
state.system_prompt_text = build_system_prompt_text(&state.project_tree);
```

**位置 3**：普通消息发送路径（约行 331-340）— 删除日期变化检测逻辑，改为在首条消息注入日期。

当前逻辑：
```rust
// 检查日期是否变化，需要重建 system prompt          ← 删掉整个 if-else
let today = chrono_date();                           ← 删掉
let system_text = if today != state.system_prompt_cached_date {  ← 删掉
    state.system_prompt_text =                                   ← 删掉
        build_system_prompt_text(&state.project_tree, &today);   ← 删掉
    state.system_prompt_cached_date = today.clone();             ← 删掉
    state.system_prompt_text.clone()                             ← 删掉
} else {                                                         ← 删掉
    state.system_prompt_text.clone()                             ← 删掉
};                                                               ← 删掉

// 构建带缓存断点的消息列表                             ← 保留
let mut messages = state.messages.clone();             ← 保留
apply_cache_breakpoints(&mut messages);                ← 保留

// 构建 system prompt                                  ← 保留
let system_content = build_system_content(&system_text);  ← 保留，但 system_text 来源变了
```

改为（精简后 + 首条消息注入日期）：
```rust
// 首条消息注入日期（替代原来写在 system prompt 的做法）
// 此时 state.messages 已包含刚 push 的 user message
if state.messages.len() == 1 {
    let today = chrono_date();
    state.messages[0].content.push_str(
        &format!("\n\n[Current date: {}]", today)
    );
}

let system_text = state.system_prompt_text.clone();  // 直接取，无需每日重建
let mut messages = state.messages.clone();
apply_cache_breakpoints(&mut messages);
let system_content = build_system_content(&system_text);
```

**位置 4**：技能执行路径（约行 331-340 之前，技能发送部分 ~line 290-300）— 取 `system_text` 的方式同样简化。
```rust
// 改前
let system_text = state.system_prompt_text.clone();
// 不改，这行已经是对的。技能路径不需要注入日期（首条消息早已注入过）。
```

**位置 5**：`AppState` 结构体（约行 17-39）— 删除 `system_prompt_cached_date` 字段（不再需要）。
```rust
// 删除这一行：
pub system_prompt_cached_date: String,
```

同时删除初始化处的对应行（约行 84）：
```rust
// 删除这一行：
system_prompt_cached_date: String::new(),
```

以及删除 `run()` 函数中紧随初始化后的赋值（约行 91）：
```rust
// 删除这一行：
state.system_prompt_cached_date = today;
```

**注意**：`chrono_date()` 函数保留不动，它仍然被用到（首条消息注入时调用一次）。

**预期效果**：system prompt 缓存跨天存活，避免每天首次请求的 ~500 token 重建开销。

#### 子任务 3：缓存命中率公式修正

**文件**：[ui/draw.rs](src/ui/draw.rs) — `draw_status_bar()`

**当前公式**（行 213-218）：
```rust
let cache_total = state.total_cache_creation_tokens + state.total_cache_read_tokens;
let hit_rate = state.total_cache_read_tokens as f64 / cache_total as f64 * 100.0;
```

**问题**：`cache_creation` 持续累积，分母越来越大，命中率虚低且持续下降。

**改为**：
```rust
// API 返回的 input_tokens 已扣除缓存命中部分。还原实际输入规模：
// effective_input = API计费的input + 缓存白送的部分
let effective_input = state.total_input_tokens + state.total_cache_read_tokens;
let hit_rate = if effective_input > 0 {
    state.total_cache_read_tokens as f64 / effective_input as f64 * 100.0
} else {
    0.0
};
```

**说明**：`input_tokens` 是 API 扣掉缓存命中后实际计费的 token。新公式算出"缓存帮我省了多少比例的输入成本"，更直观。

---

#### 实施后验证

改完编译通过后，按以下步骤确认效果：

1. `cargo run` 启动，发一条简短消息
2. 等回复完成，查看状态栏 `♻` 数值（首次请求为 0%，正常）
3. 再发第二条消息，回复完成后 `♻` 应跳到 30-60%（system + messages[0] 命中）
4. 继续对话到 5-6 轮，`♻` 应稳定在 70% 以上
5. 如果看到 `♻` 持续在 20% 以下，说明缓存未命中——检查 API 服务端是否支持多断点（MiMo Token Plan 基于 Anthropic 协议，应支持）

> 如果 MiMo API 不支持多断点（只会用第一个 `cache_control`），回退方案：只保留 `messages[0]` 断点 + 移除 `step_by` 循环。单断点 + 去日期仍能将命中率从 15% 提升到 40-55%。

---

### P0-1：代码块语法高亮 ✅ 已完成

**目标**：markdown 代码块按语言着色，代码一眼可读

**改动**：[ui/draw.rs](src/ui/draw.rs)、[Cargo.toml](Cargo.toml)

**方案**：
- 引入 `syntect` crate（~1MB 增量）
- `draw_chat_area` 中检测 ` ``` ` 围栏，围栏内用 `syntect::easy::HighlightLines` 逐行着色
- 复用 [theme.rs](src/ui/theme.rs) 中已定义的 5 个 `CODE_*` 颜色常量
- 流式 buffer 中未闭合的代码块（` ``` ` 开始但未结束）同样着色
- 语言标签显示在代码块顶部边框（`┌─ rust ─┐`）

**syntect token → 颜色映射**：Keyword→`CODE_KW` | String→`CODE_STR` | Function→`CODE_FN` | Comment→`CODE_COMMENT` | 背景→`CODE_BG`

实现后移除 [theme.rs](src/ui/theme.rs) 的 `#[allow(dead_code)]`。

---

### P0-2：输入历史浏览 ✅ 已完成

**目标**：↑/↓ 浏览已发送的消息，不必重新输入

**改动**：[app.rs](src/app.rs)

**方案**：
- `AppState` 新增 `input_history: Vec<String>` 和 `history_idx: Option<usize>`
- 每次 Ctrl+Enter 发送后 push 到 history
- ↑ 回退到上一条，↓ 前进到下一条
- 编辑历史消息后 Enter 发送的是修改版（不覆盖历史）

---

### P1-0：聊天区滚动浏览 ✅ 已完成

**目标**：对话长了之后能回看历史消息，不被强制滚到底

**改动**：[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)

**现状问题**：`draw_chat_area` 始终 `scroll = total_lines - visible_height`，用户无法回看历史。长对话中想检查之前的代码或回答只能重新发一遍。

**方案**：
- `AppState` 新增 `chat_scroll: usize`（0 = 底部，越大越靠上）
- PageUp/PageDown 翻页（半屏），Ctrl+Home 回到底部
- 新消息到达时自动重置到底部（`chat_scroll = 0`）
- 状态栏显示 `[↑3]` 表示向上偏移了 3 行

---

### P1-1：消息搜索 ✅ 已完成

**目标**：Ctrl+F 在对话中搜索关键词，高亮匹配项并跳转

**改动**：[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)

**方案**：
- Ctrl+F 进入搜索模式，底部输入框变为搜索框
- 实时高亮匹配项（黄色背景），Enter/Shift+Enter 上下跳转
- Esc 退出搜索，恢复正常输入

---

### P1-2：启动性能优化 ✅ 已完成

**目标**：减少首次渲染延迟

**改动**：[ui/draw.rs](src/ui/draw.rs)

**现状问题**：`build_theme()` 在每次 `draw()` 调用时执行 `ThemeSet::load_defaults()`，每次都从内置数据解析主题。应和 `syntax_set()` 一样用 `OnceLock` 缓存。

**方案**：
```rust
fn theme() -> &'static Theme {
    static T: OnceLock<Theme> = OnceLock::new();
    T.get_or_init(build_theme)
}
```

---

### P1-3：文件操作系统 ✅ 已完成

**目标**：MiMo 直接读/写/编辑项目文件，从"聊天工具"升级为"编码助手"

**改动**：[app.rs](src/app.rs)、新增 [src/file_ops.rs](src/file_ops.rs)、[src/main.rs](src/main.rs)

**三个核心命令**：

| 命令 | 功能 | 示例 |
|------|------|------|
| `/read <path>[:start[-end]]` | 读取文件注入上下文（带行号） | `/read src/main.rs:42` 或 `/read src/main.rs:10-50` |
| `/write <path>` | 提取最近代码块写入文件 | `/write src/foo.rs` |
| `/edit <path> <old> <new>` | 精确字符串替换 | `/edit src/main.rs fn_old fn_new` |

**安全约束**：
- 路径沙箱：禁止 `../` 穿越工作目录、禁止绝对路径、`.git/` 不可写
- Windows 兼容：`canonicalize` 的 `\\?\` 前缀自动去除
- 自动备份：写入前备份原文件到 `.mimo-opt/backups/`（保留最近 5 个版本）
- 写入后显示 `✓ 已写入 src/foo.rs (1234 B)`

---

### P1-4：重要操作确认机制 ✅ 已完成

**目标**：文件覆盖/删除/清空对话等破坏性操作弹出确认框，防止误操作

**改动**：[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)

**触发确认的操作**：

| 操作 | 触发条件 | 确认内容 |
|------|---------|---------|
| 文件写入 | `/write`（文件已存在） | 文件路径 + 代码块预览 |
| 文件编辑 | `/edit` | 文件路径 + 替换次数 + 文本详情 |
| 清空对话 | `/clear` | 消息数量 |
| 退出 | Ctrl+Q 且有对话 | 消息数量提示 |

**确认 UI**：居中弹窗覆盖对话区，红色边框，`y` 确认 / `n` 取消 / `d` 展开详情

**技术实现**：
- `ConfirmAction` 枚举：WriteFile / EditFile / ClearChat / Quit
- `ConfirmState` 结构体：action + detail（预览文本）
- `execute_confirm()` 函数：y 键触发，执行实际操作
- `show_confirm_detail()` 函数：d 键触发，显示详情预览
- 两阶段执行：Phase 1 验证+准备 → 设置 pending_confirm → Phase 2 确认后执行
- confirm 活跃时屏蔽 Ctrl+Enter / Ctrl+F / 字符输入，防止误操作
- `/write` 代码块提取在 Phase 1 完成，确认时代码已缓存

---

### P1-5：技能系统增强 ✅ 已完成

**目标**：技能支持参数、内置默认命令、输出自动截断、输入时提示可用技能

**改动**：[config.rs](src/config.rs)、[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)

**四项子任务（一次改完）**：

1. **参数传递** — 命令模板支持 `{args}` 占位符：`/lint --fix` → 执行 `cargo clippy --fix 2>&1`
2. **内置默认** — 配置文件 skills 为空时自动填入 `/lint` `/test` `/build` `/git` `/diff`
3. **结果增强** — 输出超过 2000 字符截断 + 执行耗时显示 + stderr/stdout 区分
4. **输入提示** — 输入 `/` 时在输入框上方浮动显示可用技能列表；`/help` 列出全部

---

### P1-6：消息结构升级（Content enum）✅ 已完成

**目标**：`ChatMessage.content` 从纯 `String` 改为 `enum Content`，为 MCP/工具调用/图片铺路

**改动**：[api/types.rs](src/api/types.rs)、[api/mod.rs](src/api/mod.rs)、[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)

```rust
#[serde(untagged)]
pub enum Content {
    Text(String),
    // 预留：Image, ToolUse, ToolResult
}
```

`#[serde(untagged)]` 保证 API 兼容（纯字符串序列化不变）。`Content` 提供 `text()`、`as_str()`、`as_mut_str()` 三个辅助方法。全部 17 处 ChatMessage 创建和 7 处 content 访问已迁移。现在改成本低，以后加 Image/ToolUse 只需加枚举变体。

---

### P2-7：多会话持久化 ✅ 已完成

**目标**：会话自动保存，重启可恢复，不丢上下文

**改动**：[app.rs](src/app.rs)、新增 [src/session.rs](src/session.rs)、[ui/draw.rs](src/ui/draw.rs)、[src/main.rs](src/main.rs)

**Session 结构**：`id`（时间戳）+ `name` + `messages` + `created_at` + `updated_at`

**存储**：`~/.config/mimo-opt/sessions/{id}.json`，JSON 序列化

**自动行为**：
- 启动时加载最近会话（`Session::list()` 按 `updated_at` 降序取第一条）
- 每完成 5 条消息自动保存（`StreamResult::Done` 中触发）
- 退出时自动保存（Ctrl+Q 路径 + `execute_confirm(Quit)` 路径）

**手动操作**：
- Ctrl+N：保存当前会话 → 创建新会话 → 清空对话和计数器
- F2：保存当前会话 → 切换到列表中下一个会话（循环）

**标题栏**：显示 `MiMo-OPT [session_name]`，名称超 20 字符截断

---

### P2-8：终端/桌面端双模式

**目标**：用户按需选择终端 TUI 或桌面 GUI，一套代码两种运行模式

**改动**：新增 `src/platform/` 模块，可选依赖 `tauri`

**方案**：

- **终端模式**（默认）：`mimo-opt` → 当前 ratatui TUI，零额外依赖
- **桌面模式**：`mimo-opt --gui` 或单独的 `mimo-opt-gui` → Tauri + WebView 桌面窗口
- 核心逻辑（api/、config/、app.rs 的消息处理）抽到 `src/core/`，TUI 和 GUI 各自是壳
- Cargo.toml 用 feature flag 控制：`cargo run` 默认 TUI，`cargo run --features gui` 启用桌面端

**分层架构**：
```
src/
├── core/           # 平台无关：API 客户端、消息处理、技能系统、文件操作
├── ui/             # TUI 壳（ratatui + crossterm）
├── gui/            # GUI 壳（tauri，feature = "gui" 时编译）
├── platform/       # 平台适配（剪贴板、通知、文件对话框）
├── config.rs, app.rs, main.rs  # 入口路由
```

**不重复造轮子**：GUI 模式复用 core 层全部逻辑，只替换渲染层。TUI 优先开发，GUI 作为 feature flag 可选编译。

---

### P3-9：代码块复制功能 ✅ 已完成

**目标**：Ctrl+Y 复制最后一个代码块到剪贴板

**改动**：[app.rs](src/app.rs)、[ui/draw.rs](src/ui/draw.rs)、[Cargo.toml](Cargo.toml)

- 新增依赖：`cli-clipboard = "0.4"`
- `AppState` 新增 `code_blocks: Vec<(String, String)>` 和 `copy_status: Option<String>`
- `collect_code_blocks(state)`：扫描所有消息，提取代码块的 (lang, content) 列表
- Ctrl+Y：调用 `collect_code_blocks` → `cli_clipboard::set_contents()` 复制最后一个代码块
- 状态栏显示 "✓ 已复制 rust 代码块 (12 行)"，下次按键自动消失
- 无代码块时显示 "对话中未找到代码块"

---

### P3-10：技能管理 ✅ 已完成

**目标**：在 TUI 内增删技能，不用手动编辑 config.json

**改动**：[app.rs](src/app.rs)、[config.rs](src/config.rs)

**三个命令**：
- `/skills` — 显示格式化技能列表（名称 → 命令模板）
- `/addskill <name> <cmd>` — 添加技能并自动保存 config.json
- `/rmskill <name>` — 删除技能并自动保存 config.json

`Config` 新增 `save()` 方法：序列化为 JSON 写入配置文件。

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

**多断点为何安全**：多个 `cache_control` 标记只影响 API 服务端的缓存切分策略。每个断点告诉 API "这个位置之前的全部内容可以作为缓存前缀"。API 收到的消息数组完整无缺，模型推理时看到的上下文与无缓存完全一致。多个断点只是让 API 有更多层级的缓存可供命中，不会让模型"只看到缓存部分"。

**System prompt 去日期为何安全**：日期信息移入首条 user message 后，模型仍然知道当前日期。唯一变化是日期的载体从 system 变成了 user，这对模型推理质量无影响（Anthropic API 中 system 和 user 在注意力机制中没有优先级差异）。

### 技能安全

- 技能是用户自行配置的 shell 命令，等同于用户手动在终端执行
- 不做沙箱隔离（程序员需要完整 shell 访问）
- 执行超时无硬限制（长命令如 `cargo build` 需要时间）
- 技能输出直接发给 MiMo 分析，不存储到文件

### 文件操作安全

- 所有文件路径必须位于当前工作目录内，禁止 `../` 穿越、禁止绝对路径
- 写入操作自动备份到 `.mimo-opt/backups/`（保留最近 5 个版本）
- `.git/` 目录禁止任何写入操作
- 破坏性操作（覆盖、删除）必须经过确认对话框，用户主动选择 `y` 才执行
- 不做文件系统以外的操作（不访问网络、不修改注册表、不操作进程）

### 桌面端原则

- TUI 和 GUI 共享同一套 core 层代码，不重复实现业务逻辑
- TUI 优先开发，GUI 作为 feature flag 可选编译
- 桌面端不做 WebView 内嵌浏览器式的"套壳网页"，保持原生性能

---
## 安全审计（甲方高级工程师审查，2026-05-19）

> 以下问题按风险和影响从高到低排列，**P0 必须在上线前修复**。

---

### 🔴 P0-1：API Key 可能通过错误消息泄漏

**文件**：[api/mod.rs](src/api/mod.rs):116-118, [api/mod.rs](src/api/mod.rs):92-94, [app.rs](src/app.rs):237-238

**问题**：API 请求失败时，服务端返回的原始 body 被完整放入 error message：
```rust
let body = response.text().await.unwrap_or_default();
anyhow::bail!("{} {}", status, body);
```
该 error 随后被存入 `state.api_error_detail` 并在标题栏展示。如果 MiMo API 因鉴权失败而在响应体中回显了 API Key，用户截图或屏幕共享时会泄漏密钥。

**修复方案**：
```rust
let body = response.text().await.unwrap_or_default();
let truncated: String = body.chars().take(200).collect();
anyhow::bail!("HTTP {}: {}", status, truncated);
```

---

### 🔴 P0-2：配置文件 API Key 明文存储，文件权限未加固

**文件**：[config.rs](src/config.rs):54-55, [config.rs](src/config.rs):89-90

**问题**：API Key 以明文 JSON 写入 `~/.config/mimo-opt/config.json`，未显式设置文件权限。在 Linux/macOS 多用户环境下，若 umask 宽松，其他用户可能读取该文件。

**修复方案**（仅 Unix）：
```rust
use std::os::unix::fs::PermissionsExt;

std::fs::write(&path, content)?;
#[cfg(unix)]
{
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
}
```

---

### 🔴 P0-3：技能系统存在命令注入风险 ✅ 已修复 v0.3.2

**文件**：[app.rs](src/app.rs):542-547

**修复**：添加 `shell_escape()` 函数，对 `{args}` 替换做平台适配的 shell 转义：
- Unix/sh：单引号包裹 + 内部单引号转义
- Windows/cmd：`^` 转义特殊字符（`%^&<>|"()!`）

**问题**：`{args}` 占位符替换是裸字符串替换，无任何转义或校验：
```rust
let cmd = if cmd_tpl.contains("{args}") {
    cmd_tpl.replace("{args}", &args)  // 直接拼接，无转义
};
```
随后通过 `cmd /C` 或 `sh -c` 执行。若用户输入 `/skill_name $(malicious)` 且 skill 模板包含 `{args}`，shell 将执行注入的命令。

**缓解因素**：技能配置由用户自己管理（不被外部控制），但仍存在误操作风险和配置文件被恶意修改的隐患。

**修复方案**（按推荐度排序）：
1. **推荐**：避免 `sh -c` 方式，改为直接传参：
```rust
let parts: Vec<&str> = cmd_tpl.split_whitespace().collect();
let resolved: Vec<String> = parts.iter()
    .map(|p| p.replace("{args}", &args))
    .collect();
let mut child = tokio::process::Command::new(&resolved[0]);
for arg in &resolved[1..] {
    child.arg(arg);
}
```
2. **次选**：对 args 做 shell 转义（过滤 `$()` {} ; | & 等特殊字符）
3. **文档化**：在 README 中明确标注技能执行的 shell 上下文和安全假设

---

### 🟡 P1-4：Config Debug 实现暴露 API Key ✅ 已修复 v0.3.1

**文件**：[config.rs](src/config.rs):4

**修复**：手动实现 `Debug` + `mask_key()` 脱敏函数，`{:?}` 输出 `tp-1...cdef` 格式。

**问题**：`Config` 原本派生了 `Debug`：
```rust
#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
```
任何 `dbg!(&config)` 或 `{:?}` 打印都会明文输出 API Key。若未来引入日志框架且记录 debug 级别配置，极易泄漏到日志文件。

**修复方案**：手动实现 `Debug`，对 `api_key` 做脱敏：
```rust
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("api_key", &mask_key(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("auth_type", &self.auth_type)
            .field("skills", &self.skills)
            .finish()
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 { return "***".into(); }
    format!("{}...{}", &key[..4], &key[key.len()-4..])
}
```
然后移除 `#[derive(Debug)]` 中的 Debug，或使用 `#[derive(Debug)]` + 字段级自定义。

---

### 🟡 P1-5：会话文件明文存储完整对话历史 ✅ 已修复 v0.3.1

**文件**：[session.rs](src/session.rs):53-59

**修复**：`save()` 已对 Unix 平台设置 0600 权限（同 config 加固方案）。

**远期可考虑**：AES-256-GCM 会话加密（密钥从 OS keychain 读取）

---

### 🟡 P1-6：tokio::spawn 任务无取消机制，资源泄漏 ✅ 已修复 v0.3.2

**文件**：[app.rs](src/app.rs)

**修复**：3 个 spawn 点全接入 `tokio::select!` + `oneshot` channel，Ctrl+C / Esc 发送取消信号真正中止 HTTP 请求。

---

### 🟡 P1-7：SSE 流式通道无背压，潜在内存膨胀 ✅ 已修复 v0.3.1

**文件**：[app.rs](src/app.rs):153

**修复**：`unbounded_channel()` → `channel(256)`，有界通道防止无限堆积。

---

### 🟡 P1-8：消息历史无上限，长对话 OOM + 超上下文窗口 ✅ 已修复 v0.3.2

**文件**：[app.rs](src/app.rs)

**修复**：`maybe_truncate_messages()` — 超 200 条自动截断保留最近 150 条，160 条时状态栏警告，状态栏显示 `✉ N` 计数。

---

### 🔵 P2-9：无请求重试机制

**文件**：[api/mod.rs](src/api/mod.rs):99-220

**问题**：网络瞬时故障（timeout、connection reset）直接报错给用户，无自动重试。

**修复方案**：对幂等错误（5xx、timeout、connection error）进行最多 3 次重试，间隔 1s/2s/4s 指数退避。

---

### 🔵 P2-10：unwrap_or_default() 静默吞掉错误

**文件**：[session.rs](src/session.rs):24-28, [app.rs](src/app.rs):多处

**问题**：`std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()` 在系统时钟异常时返回 `Duration::ZERO`，导致时间戳为 0，影响会话排序。虽概率极低，但发生时毫无提示。

**修复方案**：引入 `log` crate，至少记录 warning：
```rust
let ts = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_else(|_| {
        log::warn!("系统时钟异常，使用默认时间戳 0");
        Duration::from_secs(0)
    })
    .as_secs();
```

---

### 🔵 P2-11：app.rs 过于庞大（~1500 行），缺乏模块分层

**文件**：[app.rs](src/app.rs) — 1501 行

**问题**：单文件混合了事件循环、键盘处理、技能执行、文件命令、确认对话框、缓存策略、搜索、代码块提取、system prompt 构建、项目文件扫描、费用计算、日期计算等职责。维护和测试困难。

**建议拆分**：
```
src/
├── app.rs              # 事件循环 + AppState (~400 行)
├── commands/
│   ├── mod.rs
│   ├── files.rs        # /read /write /edit 命令
│   ├── skills.rs       # 技能执行 + 管理
│   └── search.rs       # Ctrl+F 搜索逻辑
├── prompt.rs           # system prompt 构建 + 缓存断点
├── scanner.rs          # 项目文件扫描
├── confirm.rs          # 确认对话框状态机
└── cost.rs             # 费用计算 + 日期工具
```

---

### 🔵 P2-12：缺少结构化日志框架

**问题**：整个项目只有 `eprintln!` 和 `error_message` 字段做错误报告，没有日志级别、模块过滤、文件输出能力。线上排查问题时完全依赖界面截图。

**修复方案**：引入 `log` + `env_logger`，在关键路径加日志（API 请求/响应脱敏、缓存命中/失效、文件操作、技能执行、会话保存）。设置 `RUST_LOG=mimo_opt=debug` 启用。

---

### 🔵 P2-13：零测试覆盖

**问题**：项目无任何单元测试或集成测试文件。对于处理用户输入、文件操作、API 响应的工具，缺乏测试是高风险项。

**建议优先测试的模块**：

| 模块 | 测试内容 |
|------|---------|
| `file_ops` | 路径验证（穿越检测、绝对路径拒绝、.git 保护）、行范围解析、备份轮转 |
| `api/types` | Content enum 序列化/反序列化往返、CacheControl 序列化格式 |
| `config` | 默认值填充、JSON 读写往返、API Key 脱敏 |
| `prompt` | `apply_cache_breakpoints` 断点位置计算 |
| `cost` | `calculate_cost` 边界值（0 token、大数） |
| `session` | Session 序列化/反序列化、list 过滤非 JSON 文件 |

---

### 🔵 P2-14：scan_project_tree 存在死参数

**文件**：[app.rs](src/app.rs):1422-1448

**问题**：`scan_dir_recursive()` 接受 `is_git_repo: bool` 参数但从未使用。该参数在整个递归链路中传递但无任何逻辑引用——疑似预留但未实现的 `.gitignore` 解析功能。

**修复方案**：删除该参数，或在后续迭代中实现 `.gitignore` 解析。

---

### 🔵 P2-15：每次发送消息都 clone 整个 messages 数组

**文件**：[app.rs](src/app.rs):677, 1392

**问题**：发送路径每次执行 `state.messages.clone()`，长对话（100+ 条）下开销明显。`ChatMessage.content` 是 `String`，clone 成本随内容线性增长。

**修复方案**：改用 `Arc<Vec<ChatMessage>>` 共享消息列表，仅在追加新消息时才 clone。或改为在发送前原地修改、发送后还原。

---

### 🔵 P2-16：cli-clipboard 在 Wayland 环境可能不可用

**文件**：[Cargo.toml](Cargo.toml):20

**问题**：`cli-clipboard = "0.4"` 在 Wayland 原生环境下可能无法访问剪贴板（需 `wl-clipboard` 或 XWayland 桥接）。

**修复方案**：改用 `arboard` crate（更现代，自动适配 X11/Wayland/macOS/Windows），或在复制失败时提示用户安装系统剪贴板工具。

---

### 🟢 P3-11：API 探测请求浪费配额

**文件**：[api/mod.rs](src/api/mod.rs):74-83

**问题**：启动时发 `"hi"` 消息探测 API，实际消耗 token 配额。虽微不足道，但可以更优雅。

**优化方案**：跳过启动探测，首条实际消息发送时自然会检测 API 可用性，失败时有足够的错误信息。

---

### 🟢 P3-12：output_tokens 流式中间计数不精确

**文件**：[api/mod.rs](src/api/mod.rs):170

**问题**：`output_tokens += 1` 假设每个 `text_delta` 事件恰好含 1 个 token。实际上一个 delta 可能包含多个 token。真正的计数值在 `message_delta` 的 `usage.output_tokens` 中。

**优化方案**：去掉 `output_tokens += 1` 的粗略累加，流式过程中显示 "..."，只在 `StreamResult::Done` 时用准确的 usage 更新。

---

### 🟢 P3-13：对话轮次分隔线（UI_DESIGN 已规划未实现）

**文件**：[ui/draw.rs](src/ui/draw.rs):218-303

**问题**：[UI_DESIGN.md](UI_DESIGN.md) 设计稿中不同轮次对话之间有虚线分隔线（`─ ─ ─ ─ ─`），当前实现中所有消息连续排列，长对话中难以快速定位轮次边界。

**实现方案**：
- 在 `draw_chat_area()` 中，每条消息（非第一条）前插入虚线分隔线
- 分隔线颜色使用 `Theme::CODE_BORDER`（#3b4261），保持低调
- 分隔线样式：`" ─".repeat(max_width / 2)` 形成虚线

---

### 🟢 P3-14：会话侧边栏（Ctrl+B 呼出，UI_DESIGN 已规划未实现）

**文件**：[ui/draw.rs](src/ui/draw.rs)、[app.rs](src/app.rs)

**问题**：[UI_DESIGN.md](UI_DESIGN.md) 设计了侧边栏（`Ctrl+B` 呼出），显示会话列表、当前会话高亮、新建会话入口。当前只能通过 F2 盲切会话，无法一览全部会话。

**实现方案**：
- `AppState` 新增 `sidebar_open: bool`
- 侧边栏宽度固定 24 字符，左侧显示
- 内容：标题 "sessions" + 会话列表（当前会话前缀 `▸` + 亮色）+ 底部分隔线 + `+` 新建按钮
- Ctrl+B 切换开/关，对话区自动缩窄
- 侧边栏边框使用直角（`┌─ ─┐`），与代码块风格一致

**参考设计（UI_DESIGN.md:135-155）**：
```
 ┌─ sessions ──────┐
 │                  │
 │ ▸ Rust HTTP 服务器   ← 当前会话，亮色
 │   解释 spawn 作用
 │                  │
 │   调试 API 超时
 │   部署脚本优化
 │   代码审查
 │                  │
 │ ─────── + ─────  │  ← 新建会话
 │                  │
 └──────────────────┘
```

---

### 🟢 P3-15：代码块深色背景（UI_DESIGN 已规划未实现）

**文件**：[ui/draw.rs](src/ui/draw.rs):267-278

**问题**：当前代码块只有左侧竖线边框（`│`），没有背景色填充。[UI_DESIGN.md](UI_DESIGN.md) 配色方案中定义了代码块背景色 #1a1b26。加上背景可以更强地区分代码和对话文字。

**实现方案**：
- 在 `draw_chat_area()` 中，代码块行使用 `Span::styled` 设置 `bg = Color::Rgb(0x1a, 0x1b, 0x26)`
- 代码块左边框 `│` 也使用该背景色保持一致
- 注意与 Tokyo Night 终端默认背景协调

---

### 🟢 P3-16：终端最小尺寸警告 ✅ 已完成 v0.3.5

**文件**：[app.rs](src/app.rs):166-173

**问题**：[UI_DESIGN.md](UI_DESIGN.md) 定义了最小终端尺寸要求（60×20），当前未做检查。窗口过小时界面布局会崩坏（标题栏文字重叠、输入框被截断）。

**实现方案**：
- 在 `run_loop()` 的每次 `terminal.draw()` 前检查 `f.area()` 尺寸
- 宽度 < 60 或高度 < 20 时，整个画面渲染一条居中警告：`"窗口过小，请调整到 60×20 以上"`
- 或者仅在 `AppState` 中设置 `min_size_warning: bool` 并在状态栏用红色显示

---

### 🟢 P3-17：引用块（blockquote）视觉支持

**文件**：[ui/draw.rs](src/ui/draw.rs)、[ui/theme.rs](src/ui/theme.rs)

**问题**：[UI_DESIGN.md](UI_DESIGN.md) 配色中定义了引用块边框色 #3d59a1（`BORDER_BLOCKQUOTE`），但 [theme.rs](src/ui/theme.rs) 中没有对应常量，[draw.rs](src/ui/draw.rs) 中也未实现 markdown `>` 引用块的渲染。MiMo 回复中引用块显示为普通文本，无视觉区分。

**实现方案**：
- `theme.rs` 新增 `pub const BLOCKQUOTE_BORDER: Color = Color::Rgb(0x3d, 0x59, 0xa1);`
- `draw_chat_area()` 中检测以 `>` 开头的行，用 `│` 竖线 + 缩进 + 特定颜色渲染
- 连续 `>` 行合并为一个引用块

---

### Phase 11 去重 + API 架构预留 ✅ 已完成

#### 代码去重（6 项）

| # | 去重项 | 之前 | 之后 | 文件 |
|---|--------|------|------|------|
| 1 | 时间戳 `now_secs()` | 5 处内联 | `util::now_secs()` | util.rs, session.rs, file_ops.rs, app.rs |
| 2 | 会话保存 `save_session()` | 5 处复制粘贴 | 1 函数 5 调用 | app.rs |
| 3 | 工作目录 `get_cwd()` | 5 处 match + 错误格式化 | `util::get_cwd()` | util.rs, app.rs |
| 4 | 取消消息 `push_cancelled_message()` | 2 处相同 ChatMessage 创建 | 1 函数 2 调用 | app.rs |
| 5 | `SystemContent` 缺 Clone | 编译时 `.to_vec()` 不通过 | 新增 `Clone` derive | api/types.rs |
| 6 | 新增 `src/util.rs` | — | 共享工具模块 | util.rs |

#### API 多格式架构预留

**已搭好骨架，暂未接入第三方 API**：

- `Config` 新增 `api_format`（`"anthropic"` | `"openai"`）和 `max_tokens` 配置项
- `MiMoClient` 根据 `api_format` 自动选择端点和请求/响应格式
- Anthropic 格式：`/v1/messages` + AnthropicRequest/StreamEvent 解析（不变）
- OpenAI 格式：`/v1/chat/completions` + OpenAIRequest/StreamEvent 解析
- 两套流式解析器独立实现，互不干扰

**设计原则：防止"多而不精"**：

| 原则 | 说明 |
|------|------|
| **独立实现** | 每种 API 格式有独立的 `build_xxx_request()` + `stream_xxx()` 方法对，不共用解析逻辑 |
| **可选启用** | 默认 `api_format: "anthropic"` 不变，旧用户零感知 |
| **渐进扩展** | 新增格式只需加方法对，不改已有代码路径 |
| **格式绑定** | 认证方式（auth_type）和数据格式（api_format）正交，4 种组合自由搭配 |
| **验证门槛** | 接入新 API 前必须完成：流式解析测试 + 错误处理 + token 统计准确性验证 |

**当前支持的 API**：

| API | auth_type | api_format | base_url | 状态 |
|-----|-----------|------------|----------|------|
| MiMo Token Plan | anthropic | anthropic | token-plan-sgp.xiaomimimo.com/anthropic | ✅ 默认 |
| DeepSeek | bearer | openai | api.deepseek.com | ✅ 已实测通过 |
| OpenAI | bearer | openai | api.openai.com | ✅ 已集成 |
| MiMo API | bearer | openai | api.xiaomimimo.com/v1 | ✅ 可用 |
| GLM (智谱) | bearer | openai | open.bigmodel.cn/api/paas/v4 | ⏳ provider: "custom" |
| 通义千问 | bearer | openai | dashscope.aliyuncs.com/compatible-mode/v1 | ⏳ provider: "custom" |

**待做（每个 API 单独验证后再标记可用）**：
- [ ] GLM 流式解析实测（注意其 SSE 格式可能有细微差异）
- [ ] 通义千问实测
- [ ] 各 API 的错误响应格式适配（不同厂商的 error body 结构不同）
- [ ] 非标准 API 的特殊处理模块（如需单独一套逻辑的厂商）

---

## 拉分方案：从 6.2 到 8.5+ 的路线图

> 目标：将项目综合评价从 **6.2/10** 提升到 **8.5/10（优秀）**。
> 以下每一项都标注了对评分维度的贡献分和具体实现路径。

---

### 第一梯队：基础补课（+1.25 分，2-3 周）

这三项是当前最大的失分项，做完项目就能从 6.2 拉到 7.5。

---

#### 🎯 T-1：测试体系建设（测试覆盖 0.0 → 7.0，贡献 +0.70）

**为什么是最大杠杆**：10 个维度中测试权重 10%，当前得 0 分，白丢 1.0 分。补到 7.0 就净赚 0.7 分。

**文件**：新增 `tests/` 目录、各源文件内 `#[cfg(test)]` 模块

**实施步骤**：

**Step 1 — 单元测试（1 天）**：
```rust
// src/file_ops.rs 底部
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reject_absolute_path() {
        let cwd = Path::new("/home/user/project");
        assert!(validate_path("/etc/passwd", cwd).is_err());
    }

    #[test]
    fn reject_path_traversal() {
        let cwd = Path::new("/home/user/project");
        assert!(validate_path("../outside", cwd).is_err());
    }

    #[test]
    fn reject_dot_git() {
        let cwd = Path::new("/home/user/project");
        assert!(validate_path(".git/config", cwd).is_err());
    }

    #[test]
    fn allow_normal_path() {
        let cwd = Path::new("/home/user/project");
        assert!(validate_path("src/main.rs", cwd).is_ok());
    }

    #[test]
    fn reject_multiple_dot_dot() {
        let cwd = Path::new("/home/user/project");
        assert!(validate_path("sub/../../secret", cwd).is_err());
    }
}
```

```rust
// src/config.rs 底部
#[cfg(test)]
mod tests {
    #[test]
    fn apply_defaults_fills_empty_skills() { /* ... */ }
    #[test]
    fn save_and_load_roundtrip() { /* ... */ }
    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("abc"), "***");
    }
    #[test]
    fn mask_key_normal() {
        let masked = mask_key("tp-1234567890abcdef");
        assert!(masked.starts_with("tp-1"));
        assert!(masked.ends_with("cdef"));
        assert!(!masked.contains("12345678"));
    }
}
```

```rust
// src/app.rs 底部 — 纯函数易测
#[cfg(test)]
mod tests {
    #[test]
    fn calculate_cost_zero() {
        assert_eq!(calculate_cost(0, 0), 0.0);
    }
    #[test]
    fn calculate_cost_typical() {
        let cost = calculate_cost(500_000, 200_000);
        assert!(cost > 0.0);
    }
    #[test]
    fn apply_cache_breakpoints_empty() {
        let mut msgs: Vec<ChatMessage> = vec![];
        apply_cache_breakpoints(&mut msgs);
        // 不应 panic
    }
    #[test]
    fn apply_cache_breakpoints_first_has_cache_control() {
        let mut msgs = vec![
            ChatMessage { role: "user".into(), content: Content::text("a"), cache_control: None },
            ChatMessage { role: "assistant".into(), content: Content::text("b"), cache_control: None },
        ];
        apply_cache_breakpoints(&mut msgs);
        assert!(msgs[0].cache_control.is_some());
    }
    #[test]
    fn apply_cache_breakpoints_step_by_6() {
        let mut msgs: Vec<ChatMessage> = (0..20).map(|i| {
            ChatMessage { role: "user".into(), content: Content::text(format!("{}", i)), cache_control: None }
        }).collect();
        apply_cache_breakpoints(&mut msgs);
        // messages[0], [3], [9], [15] 应有 cache_control
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[3].cache_control.is_some());
        assert!(msgs[9].cache_control.is_some());
        assert!(msgs[15].cache_control.is_some());
        assert!(msgs[1].cache_control.is_none());
    }
}
```

**Step 2 — 集成测试（1-2 天）**：
```rust
// tests/api_integration.rs
// 使用 wiremock 启动本地 HTTP mock，模拟 MiMo SSE 流式响应

use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn stream_receives_all_tokens() {
    let server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(
                "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":100}}}\n\n\
                 event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n\
                 event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
            ))
        .mount(&server).await;

    let client = MiMoClient::new(
        server.uri(),
        "test-key".into(),
        "test-model".into(),
        "anthropic".into(),
    );

    let (tx, mut rx) = mpsc::unbounded_channel();
    let system = vec![];
    let messages = vec![ChatMessage {
        role: "user".into(),
        content: Content::text("hi"),
        cache_control: None,
    }];

    client.send_message_stream(&system, &messages, tx).await.unwrap();

    let mut tokens = Vec::new();
    while let Some(result) = rx.recv().await {
        match result {
            StreamResult::Token(t) => tokens.push(t),
            StreamResult::Done { .. } => break,
            StreamResult::Error(e) => panic!("unexpected error: {}", e),
        }
    }
    assert_eq!(tokens, vec!["Hello"]);
}
```

```rust
// tests/session_integration.rs
#[test]
fn session_save_and_load_roundtrip() {
    let orig = Session::new("test-session".into());
    let orig = Session {
        messages: vec![ChatMessage {
            role: "user".into(),
            content: Content::text("hello"),
            cache_control: None,
        }],
        ..orig
    };
    orig.save().unwrap();
    let loaded = Session::load(&orig.id).unwrap();
    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].content.as_str(), "hello");
}
```

**Step 3 — CI 中跑测试**（见 T-5）：
```yaml
# .github/workflows/ci.yml
- name: Run tests
  run: cargo test --verbose
```

---

#### 🎯 T-2：安全加固（安全性 5.0 → 8.5，贡献 +0.35）

**具体改动**（在已有 P0 审计项基础上补充）：

1. **API Key 优先读环境变量**（新增能力，不是替代 config.json）：
   ```rust
   // config.rs — load() 中
   let api_key = if let Ok(env_key) = std::env::var("MIMO_API_KEY") {
       env_key
   } else {
       raw.api_key.unwrap_or_default()
   };
   ```
   环境变量优先级 > 配置文件。Docker/CI/共享机器场景下不落盘。

2. **所有 session 文件设 0600 权限**（同 P0-2 的 config 加固）：
   ```rust
   // session.rs — save()
   std::fs::write(&path, content)?;
   #[cfg(unix)]
   std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
   ```

3. **请求体大小限制**（防 API 异常返回超大数据撑爆内存）：
   ```rust
   // api/mod.rs — MiMoClient::new()
   let client = reqwest::Client::builder()
       .timeout(Duration::from_secs(120))
       .pool_idle_timeout(Duration::from_secs(90))
       // 新增：限制响应体最大 10MB
       // (reqwest 通过 content-length 或流式读取截止控制)
       .build()?;
   ```

4. **敏感信息不进入日志/错误** — 全局搜索 `{:?}` 和 `Display` 确保 API Key 被脱敏。

---

#### 🎯 T-3：消除 unwrap() 和 unsafe 调用（代码质量 +0.5，贡献 +0.20）

**当前问题**：项目中至少 5 处 `unwrap()` / `expect()` 可能 panic：
```rust
// config.rs:54  — 无检查 unwrap
let dir = path.parent().unwrap();

// api/mod.rs:25 — expect
.build().expect("failed to build reqwest client");

// session.rs:26 — unwrap_or_default 静默吞错
```

**改动**：
1. `config.rs:54` → `path.parent().ok_or_else(|| anyhow::anyhow!("配置路径无父目录"))?`
2. `api/mod.rs:25` → 改为 `?` 传播，让 `main()` 处理
3. 全局审查所有 `unwrap`/`expect`/`unwrap_or_default`/`unwrap_or_else(|_| ...)` — 要么改为 `?` 传播，要么加 `log::warn!` 并走降级路径

---

### 第二梯队：结构升级（+1.05 分，4-6 周）

做完第一梯队到 7.5，这组做完到 8.5。

---

#### 🎯 T-4：app.rs 模块拆分 + 错误类型体系（架构设计 5.5 → 8.0，贡献 +0.25）

**不只是拆文件，关键是定义清晰的模块边界和错误类型**。

**拆分后的 src/ 结构**（含 trait 定义）：

```
src/
├── main.rs                  # ~30 行，入口
├── error.rs                 # 统一错误类型
├── config.rs                # 配置管理（不变）
├── app.rs                   # ~300 行，事件循环 + AppState
├── prompt.rs                # system prompt 构建 + 缓存断点
├── scanner.rs               # 项目文件扫描
├── cost.rs                  # 费用计算 + chrono_date
├── commands/
│   ├── mod.rs               # Command trait 定义 + dispatch
│   ├── files.rs             # /read /write /edit 命令
│   ├── skills.rs            # 技能执行 + 管理命令
│   ├── search.rs            # Ctrl+F 搜索
│   └── confirm.rs           # 确认对话框状态机
├── session.rs               # 会话持久化（不变）
├── api/
│   ├── mod.rs               # MiMoClient
│   └── types.rs             # API 类型（不变）
└── ui/
    ├── mod.rs               # UI 模块导出
    ├── theme.rs             # Tokyo Night 配色
    ├── draw.rs              # 主渲染（~400 行）
    ├── chat.rs              # 对话区渲染
    ├── modal.rs             # 确认弹窗 + 搜索栏
    └── widgets.rs           # 标题栏/状态栏/输入框
```

**关键：定义统一的错误类型**（不再全用 `anyhow` 字符串）：

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("配置错误: {0}")]
    Config(#[from] ConfigError),

    #[error("API 错误: HTTP {status}")]
    Api { status: u16, body: String },

    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("文件操作错误: {0}")]
    FileOp(String),

    #[error("技能执行错误: {0}")]
    Skill(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置目录不存在")]
    NoConfigDir,
    #[error("配置文件格式错误: {0}")]
    InvalidFormat(#[from] serde_json::Error),
}
```

**Command trait 定义**（为后续 MCP 铺路）：

```rust
// src/commands/mod.rs
use crate::app::AppState;
use crate::api::MiMoClient;
use tokio::sync::mpsc;

/// 所有命令（内置 / 技能 / MCP 工具）实现此 trait
#[async_trait::async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;
    async fn execute(
        &self,
        args: &str,
        state: &mut AppState,
        client: &std::sync::Arc<MiMoClient>,
        tx: &mpsc::UnboundedSender<crate::api::StreamResult>,
    ) -> Result<(), crate::error::AppError>;
}
```

---

#### 🎯 T-5：CI/CD 流水线（可维护性 5.0 → 8.0，贡献 +0.30）

**文件**：新增 `.github/workflows/ci.yml`、`rustfmt.toml`、`deny.toml`

**CI 流水线完整配置**：

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [master, main]
  pull_request:
    branches: [master, main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check + Lint + Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy (strict)
        run: cargo clippy --all-targets -- -D clippy::all -D clippy::unwrap_used

      - name: Build
        run: cargo build --verbose

      - name: Test
        run: cargo test --verbose

      - name: Doc test
        run: cargo test --doc

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  deny:
    name: License + Crate Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1
```

```toml
# rustfmt.toml
max_width = 100
tab_spaces = 4
edition = "2021"
use_small_heuristics = "Max"
newline_style = "Unix"
```

```toml
# deny.toml (cargo-deny 配置)
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "Unicode-DFS-2016"]
```

---

#### 🎯 T-6：结构化日志（可维护性 +0.10，代码质量 +0.10）

**依赖**：`Cargo.toml` 加 `log = "0.4"` + `env_logger = "0.11"`

**日志点位清单**（让后续 AI 能精确实现）：

| 位置 | 级别 | 内容 |
|------|------|------|
| `config.rs` load | info | `"加载配置: {}"` (脱敏后路径) |
| `config.rs` save | info | `"保存配置"` |
| `session.rs` load | debug | `"加载会话: {}"` (id) |
| `session.rs` save | debug | `"保存会话: {} ({} 条消息)"` |
| `session.rs` list | debug | `"列出会话: {} 个"` |
| `api/mod.rs` request | info | `"发送请求: model={}, msgs={}"` |
| `api/mod.rs` stream start | debug | `"开始接收流式响应"` |
| `api/mod.rs` stream done | info | `"流完成: input={}, output={}, cache_read={}"` |
| `api/mod.rs` stream error | warn | `"API 错误: HTTP {} {}"` (body 截断) |
| `api/mod.rs` retry | warn | `"重试 {}/3，等待 {}s"` |
| `app.rs` skill exec | info | `"执行技能: {}"` |
| `app.rs` file write | info | `"写入文件: {}"` |
| `app.rs` file edit | info | `"编辑文件: {} ({} 处)"` |
| `app.rs` session switch | info | `"切换会话: {} → {}"` |
| `app.rs` cache break | debug | `"缓存断点: {} 条消息, {} 个断点"` |
| `file_ops.rs` backup | debug | `"备份: {} → {}"` |
| `file_ops.rs` backup cleanup | debug | `"清理旧备份: 删除 {}"` |

---

#### 🎯 T-7：API 重试 + 取消 + 背压（性能优化 7.5 → 8.5，贡献 +0.10）

**组合实现**（把 P1-6、P1-7、P2-9 三个问题一次解决）：

```rust
// api/mod.rs — 新增方法
impl MiMoClient {
    pub async fn send_message_stream_with_retry(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
        tx: mpsc::Sender<StreamResult>,  // 改为有界 channel
        mut cancel: tokio::sync::oneshot::Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut last_err = None;

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = 2u64.pow(attempt as u32);
                log::warn!("重试 {}/3，等待 {}s", attempt + 1, delay);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }

            let attempt_tx = tx.clone();
            let attempt_cancel = &mut cancel;

            let result = tokio::select! {
                r = self.send_message_stream(system, messages, attempt_tx) => r,
                _ = &mut cancel => {
                    log::info!("请求被用户取消");
                    return Ok(());
                }
            };

            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // 只对幂等错误重试
                    if !is_retryable(&e) {
                        return Err(e);
                    }
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap())
    }
}

fn is_retryable(err: &anyhow::Error) -> bool {
    let s = err.to_string();
    s.contains("5")  // 5xx
        || s.contains("timeout")
        || s.contains("connection")
        || s.contains("reset")
}
```

---

### 第三梯队：差异化打磨（+0.50 分，2-4 周）

前三项让项目从"能用"变成"优秀"，这几项让项目从"优秀"变成"让人推荐"。

---

#### 🎯 T-8：MCP（Model Context Protocol）客户端支持（创新性 +0.05，功能 +0.02）

**价值**：MCP 是 Anthropic 提出的 AI 工具集成标准协议。实现 MCP 客户端后，MiMo-OPT 可以连接任何 MCP 服务端（文件系统、数据库、API 等），从"编码助手"升级为"通用 AI 代理终端"。

**实现方案**（复用 `Command` trait）：
- 新增 `src/mcp/` 模块
- 实现 MCP JSON-RPC 协议的 stdio 传输
- MCP 工具自动注册为 `/mcp_<tool_name>` 技能
- `config.json` 中配置 MCP 服务端：
  ```json
  "mcp_servers": {
    "filesystem": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"] }
  }
  ```

---

#### 🎯 T-9：会话导出为 Markdown（功能完整度 +0.02，创新性 +0.03）

**改动**：`app.rs` 新增 `/export [path]` 命令

**导出格式**：
```markdown
# MiMo-OPT 会话: session-20000_1234
> 日期: 2026/05/19 | 模型: mimo-v2-flash | Token: ↓5.2K ↑3.1K ￥0.08

---

## 用户
帮我写一个 Rust HTTP 服务器

## MiMo
好的，我来帮你实现一个基于 tokio 的异步 HTTP 服务器：

```rust
use tokio::net::TcpListener;
// ...
```
```
渲染美化后适合分享到 GitHub Issue / 博客 / 团队文档。

---

#### 🎯 T-10：主题热切换（UI/UX +0.10）

**改动**：`app.rs` + `ui/theme.rs`

- 配置文件新增 `"theme": "tokyo-night"` 字段
- 内置 3 套主题：Tokyo Night（默认）、Nord、Catppuccin
- `/theme <name>` 命令即时切换，无需重启
- `ui/theme.rs` 从硬编码常量改为 `Theme` struct，实现 `From<ThemePreset>`

**为什么性价比高**：改动量小（~100 行），但终端截图分享时视觉差异化极大，容易在社交媒体传播。

---

#### 🎯 T-11：/model 模型热切换（功能完整度 +0.03）✅ 已实现 v0.3.2

**改动**：[app.rs](src/app.rs) + [api/mod.rs](src/api/mod.rs)

- `/model <name>` 命令：运行时切换模型，不中断会话
- 标题栏模型名即时更新
- 配置自动保存

---

#### 🎯 T-12：Markdown 渲染增强（UI/UX +0.03）

**当前支持**：代码块（syntect 高亮）、普通文本

**待支持 markdown 元素**：

| 元素 | 当前 | 目标 |
|------|------|------|
| `**粗体**` | 无 | 粗体样式 |
| `*斜体*` | 无 | 斜体样式（终端支持时） |
| `~~删除线~~` | 无 | 交叉线样式 |
| `` `行内代码` `` | 无 | 反色/高亮背景 |
| `- 无序列表` | 缩进显示 | `  • ` 前缀 + 缩进 |
| `1. 有序列表` | 缩进显示 | `  1. ` 前缀 + 缩进 |
| `[链接](url)` | 显示为纯文本 | 下划线 + 青色 |
| `> 引用` | 纯文本 | 竖线 + 缩进（P3-17） |
| `--- 水平线` | 无 | 全宽分隔线 |

**实现**：在 `draw_chat_area()` 中逐行检测行首模式（`**`、`* `、`- `、`1. `、`>` 等），用 `ratatui::Span` 的 `style` 字段做样式叠加。不需要引入 markdown 解析库，正则/前缀匹配即可。

---

### 第四梯队：锦上添花（+0.15 分）

#### 🎯 T-13：Shell 集成增强

- 管道输入：`echo "帮我审查这段代码" | mimo-opt --prompt` — 直接从 stdin 读首条消息
- 单轮模式：`mimo-opt -c "这段代码有什么问题?"` — 一问一答即退出，适合脚本集成
- 退出码：API 调用成功返回 0，失败返回 1，可被 shell 脚本检查

#### 🎯 T-14：键盘快捷键自定义

- `config.json` 新增 `"keybindings"` 字段
- 支持将任意快捷键重新绑定
- 提供 `default`（类 emacs）和 `vim` 两套预设

#### 🎯 T-15：Docker 镜像发布

```dockerfile
FROM rust:1.85-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.21
COPY --from=builder /app/target/release/mimo-opt /usr/local/bin/
ENTRYPOINT ["mimo-opt"]
```

发布到 GitHub Container Registry，方便 CI 环境使用。

---

## 评分提升轨迹

| 阶段 | 完成项 | 累计分 | 说明 |
|------|--------|--------|------|
| **起点** | — | **6.2** | 当前状态 |
| 安全基线 | P0-1/2/3（审计安全修复）✅ 全部完成 | 6.6 | P0 修复是最低门槛 |
| **第一梯队** | T-1 测试 + T-2 安全 + T-3 unwrap | **7.5** | 基础补课完成 |
| 稳定基线 | P1-4/5/6/7/8 + M1/M2/M3/M4 ✅ 全部完成 | **7.7** | 安全+体验全部到位 |
| **第二梯队** | T-4 拆分 + T-5 CI + T-6 日志 + T-7 重试 | **8.3** | 工程化成熟 |
| 质量基线 | P2-9~16（审计 P2 修复） | 8.4 | 代码质量和兼容性 |
| **第三梯队** | T-8 MCP + T-9 导出 + T-10 主题 + T-11 热切换 + T-12 Markdown | **8.7** | 差异化 + 体验 |
| **第四梯队** | T-13~T-15 | **8.8** | 锦上添花 |

> **2026-05-19 状态**：P0 4/4 ✅ + P1 4/4 ✅ = 安全基线+稳定基线全部到位，当前评分 ~7.7。

---

## 汇总：审计发现 + 修复状态

| # | 优先级 | 类型 | 条目 | 状态 |
|---|--------|------|------|------|
| 1 | 🔴 P0 | 安全 | API Key 错误消息泄漏 | ✅ v0.3.1 |
| 2 | 🔴 P0 | 安全 | 配置文件权限未加固 | ✅ v0.3.1 |
| 3 | 🔴 P0 | 安全 | 技能系统命令注入风险 | ✅ v0.3.2 |
| 4 | 🟡 P1 | 安全 | Config Debug 暴露密钥 | ✅ v0.3.1 |
| 5 | 🟡 P1 | 安全 | 会话文件明文存储 | ✅ v0.3.1 |
| 6 | 🟡 P1 | 稳定性 | spawn 任务无取消机制 | ✅ v0.3.2 |
| 7 | 🟡 P1 | 稳定性 | SSE channel 无背压 | ✅ v0.3.1 |
| 8 | 🟡 P1 | 稳定性 | 消息历史无上限 | ✅ v0.3.2 |
| 9 | 🔵 P2 | 可靠性 | 无请求重试 | ⏳ |
| 10 | 🔵 P2 | 架构 | unwrap_or_default 静默吞错 | ⏳ |
| 11 | 🔵 P2 | 架构 | app.rs 超 1500 行需拆分 | ⏳ |
| 12 | 🔵 P2 | 运维 | 缺少结构化日志 | ⏳ |
| 13 | 🔵 P2 | 质量 | 零测试覆盖 | ⏳ |
| 14 | 🔵 P2 | 代码质量 | scan_project_tree 死参数 | ✅ v0.3.3 前已修复 |
| 15 | 🔵 P2 | 性能 | 每次发送 clone 整个消息列表 | ⏳ |
| 16 | 🔵 P2 | 兼容性 | cli-clipboard Wayland 不兼容 | ⏳ |
| 17 | 🟢 P3 | 优化 | check_api 浪费配额 | ✅ v0.3.5 |
| 18 | 🟢 P3 | 精度 | output_tokens 流式计数不准 | ⏳ |
| 19 | 🟢 P3 | UI | 对话轮次分隔线 | ✅ v0.3.5 |
| 20 | 🟢 P3 | UI | 会话侧边栏（Ctrl+B） | ⏳ |
| 21 | 🟢 P3 | UI | 代码块深色背景 | ✅ v0.3.4 |
| 22 | 🟢 P3 | UI | 终端最小尺寸警告 | ✅ v0.3.5 |
| 23 | 🟢 P3 | UI | 引用块（blockquote）视觉支持 | ⏳ |

### v0.3.3 Bug 修复状态

| Bug | 优先级 | 状态 | 说明 |
|-----|--------|------|------|
| B1 SSE 分隔符 | P1 | ✅ v0.3.3 前已修复 | `find_sse_separator()` 已处理 `\r\n\r\n` |
| B2 stream_buffer 扫描 | P2 | ✅ v0.3.3 前已修复 | `collect_code_blocks()` 已扫描 stream_buffer |
| B3 `/read` 行号解析 | P2 | ✅ v0.3.4 | 错误提示改善 |
| B4 send_to_mimo 消息数 | P2 | ✅ v0.3.3 前已修复 | 已有 `MAX_MESSAGES` 检查 |
| B5 is_git_repo 死参数 | P2 | ✅ v0.3.3 前已修复 | 参数已移除 |
| B6 会话 token 持久化 | P3 | ✅ v0.3.4 | Session 新增 5 个 token 字段 |
| B7 搜索高亮对比度 | P3 | ✅ v0.3.4 | 背景色 `#3b4261` → `#565f89` |
| B8 日期注入角色错误 | P3 | ✅ v0.3.4 | 改为检查 role=="user" + 不重复注入 |
| B9 27 个 clippy 警告 | P3 | ✅ v0.3.4 | 27 → 0 |

### v0.3.4 UX 优化状态

| 优化 | 优先级 | 状态 | 说明 |
|------|--------|------|------|
| U1 Ctrl+V 粘贴 | ★★★★★ | ✅ v0.3.4 | cli_clipboard::get_contents() |
| U2 代码块深色背景 | ★★★★★ | ✅ v0.3.4 | CODE_BG = #1a1b26 |
| U3 长消息发送确认 | ★★★★ | ✅ v0.3.5 | >5 行弹确认，取消恢复输入 |
| U4 对话轮次分隔线 | ★★★★ | ✅ v0.3.5 | 虚线分隔不同问答轮次 |
| U5 API 余额查询 | ★★★★ | ⏳ | — |
| U6 会话导出 Markdown | ★★★★ | ✅ v0.3.5 | `/export [path]` 导出含元信息的 .md |
| U7 输入框多行扩展 | ★★★ | ⏳ | — |
| U8 Markdown 渲染增强 | ★★★ | ⏳ | — |
| U9 错误信息历史 | ★★★ | ✅ v0.3.5 | API 错误自动记录 20 条 + `/errors` 查看 |
| U10 终端尺寸检查 | ★★★ | ✅ v0.3.5 | <60×20 居中红色警告 |

> **当前状态**：P0 全部修完 ✅ → P1 全部到位 ✅ → v0.3.3 bug 全部修复 ✅ → clippy 零警告 ✅ → U1/U2 已实现。当前可发布 v0.3.4。

---

## v0.3.3 实机测试：Bug 报告 & 用户优化建议 (2026-05-19)

> 对 v0.3.2 进行实机测试、代码审查和用户体验评估。发现 9 个 bug（含 2 个功能性 bug）+ 26 个 clippy 警告 + 15 条用户优化建议。

---

### Bug 报告

#### B1 (P1): SSE 事件分隔符硬编码 `\n\n`，不兼容 `\r\n\r\n` 服务器

**文件**: [api/mod.rs](src/api/mod.rs):237, [api/mod.rs](src/api/mod.rs):380

**问题**: 两套 SSE 解析器统一使用 `remaining.find("\n\n")` 分割事件。部分 SSE 服务器（包括某些代理和负载均衡器）使用 `\r\n\r\n` 作为事件分隔符。遇到此类服务器时，事件永远无法被分割，所有 token 静默丢失，用户看到 stream_buffer 始终为空。

**修复**: 同时匹配两种分隔符，取最先出现者：
```rust
let sep = remaining.find("\n\n")
    .or_else(|| remaining.find("\r\n\r\n"))
    .unwrap_or(remaining.len());
```

#### B2 (P2): `collect_code_blocks` 忽略 stream_buffer，Ctrl+Y 对生成中的代码块无效

**文件**: [app.rs](src/app.rs):1067-1096

**问题**: `extract_last_code_block()` 同时搜索 stream_buffer 和 messages（优先 stream_buffer），但 `collect_code_blocks()` 仅扫描 `state.messages`。用户在流式生成过程中按 Ctrl+Y 时：
- `/write` 能正确提取代码块（因为走 `extract_last_code_block`）
- Ctrl+Y 却不能（因为走 `collect_code_blocks`），显示"对话中未找到代码块"

**修复**: `collect_code_blocks()` 应同时扫描 stream_buffer，或复用 `extract_code_from_text()`。

#### B3 (P2): `handle_read_command` 行范围解析中 `unwrap_or(0)` 静默吞错

**文件**: [app.rs](src/app.rs):1127-1128

**问题**: 
```rust
let start: usize = after[..dash_idx].parse().unwrap_or(0);
let end: usize = after[dash_idx + 1..].parse().unwrap_or(0);
```
当用户输入 `/read src/main.rs:abc-def` 时静默解析为 start=0, end=0，然后条件 `start > 0 && end >= start` 为 false，路径被当作纯路径（读取整个文件）。用户以为在读指定行，实际读了整个文件。

**修复**: parse 失败时直接设置 `state.error_message` 并 return，而非静默 fallback。

#### B4 (P2): `send_to_mimo` 路径缺少消息数检查和截断

**文件**: [app.rs](src/app.rs):1478-1523

**问题**: 正常发送路径调用 `maybe_truncate_messages(state)` 检查并限制消息数。`send_to_mimo()` 在内部 clone 消息列表并追加一条 user 消息后直接发送，未做上限检查。`/read` 触发 `send_to_mimo()`，连续使用 `/read` 多次后 `msgs` 可能超过 `MAX_MESSAGES`（200），导致：
1. API 请求过大，响应变慢或直接被拒
2. 与主路径行为不一致

**修复**: 在 `send_to_mimo` 的 `msgs` 构建后添加截断逻辑。

#### B5 (P2): `is_git_repo` 死参数在全递归链路中传递但从未使用

**文件**: [app.rs](src/app.rs):1554, [app.rs](src/app.rs):1579, [app.rs](src/app.rs):1629

**问题**: `scan_dir_recursive()` 接受 `is_git_repo: bool` 参数，在整个递归中传递，但代码中无任何逻辑引用该值（clippy 已报告 `only_used_in_recursion`）。疑似为 `.gitignore` 解析预留但从未实现，造成代码噪音。

**修复**: 移除该参数，或在后续版本中实现 `.gitignore` 解析功能。

#### B6 (P3): 会话 token 计数/费用在重启后丢失

**文件**: [app.rs](src/app.rs):281-290, [session.rs](src/session.rs):8-15

**问题**: 切换会话时，代码将 `total_input_tokens` 等悉数归零（行 285-289），而非从 Session 数据中恢复。`Session` 结构体也不包含 token 累计字段。用户工作一段时间后重启或切换会话回来，状态栏 token 数和费用全部归零，看不到真实累计用量。

**修复方案**:
1. `Session` 结构体新增 `total_input_tokens` / `total_output_tokens` / `total_cache_read_tokens` / `total_cost` 字段
2. `save_session()` 同步写入这些字段
3. Ctrl+N / F2 切换时从 Session 恢复而非归零

#### B7 (P3): 搜索高亮对比度极低，几乎不可见

**文件**: [ui/draw.rs](src/ui/draw.rs):287

**问题**: 搜索匹配消息的高亮背景色为 `Color::Rgb(59, 66, 97)`（#3b4261），与终端默认深色背景（#1a1b26）仅有微弱差异。实际使用时难以辨识哪条消息被匹配到，搜索功能形同虚设。

**修复**: 改为更亮的颜色，如 `Color::Rgb(86, 95, 137)` 或金黄色 `Color::Rgb(255, 200, 50)`。

#### B8 (P3): `send_to_mimo` 在仅有一条消息时可能重复注入日期

**文件**: [app.rs](src/app.rs):1488-1495

**问题**: `send_to_mifo` 内部检查 `state.messages.len() == 1` 时注入日期。正常流程中日期已在首次普通消息发送时注入。但如果用户在全新会话中先使用 `/read`（此时 state.messages 仅包含 /read 的 assistant 结果，共 1 条），`send_to_mimo` 会在第一条消息的 content 末尾追加日期——但这条消息是 assistant 消息而非 user 消息。日期注入到了错误的消息角色中。

**修复**: 将 `len() == 1` 改为检查 messages[0].role == "user" 且 content 不含 `[Current date:]`。

#### B9 (P3): 26 个 clippy 警告积压

**文件**: 主要在 [app.rs](src/app.rs)，少量 [file_ops.rs](src/file_ops.rs), [session.rs](src/session.rs)

**分布**:
- `collapsible_match` × 9 — 内层 if 可合并到 match arm guard
- `needless_borrow` × 6 — `&client` / `&token_tx` 多余引用
- `manual_strip` × 4 — 手动切片可改用 `strip_prefix()`
- `manual_is_multiple_of` × 3 — `year % 4 == 0` → `year.is_multiple_of(4)`
- `explicit_counter_loop` × 1 — `placed` 变量可用 `.enumerate()` 替代
- `let_underscore_future` × 1 — `let _ = tx.send(...)` 未 await
- `unnecessary_sort_by` × 1 — `sort_by` → `sort_by_key`
- `only_used_in_recursion` × 1 — `is_git_repo` 死参数

---

### 用户视角优化建议

> 按体验影响和实现成本排序，`★` 越多优先级越高。

#### U1 ★★★★★ Ctrl+V 粘贴支持

**现状**: 输入框仅支持键盘逐字输入，无法粘贴剪贴板内容。用户想发送一段代码或 URL 时必须手动敲入，体验极差。现代终端工具中粘贴是基本功能。

**方案**: crossterm 支持 `KeyCode::Char('v')` + `KeyModifiers::CONTROL` 时读取剪贴板（`cli-clipboard::get_contents()`）插入到光标位置。已在依赖列表中，实现成本极低（~15 行）。

#### U2 ★★★★★ 代码块深色背景

**现状**: 代码块只有左侧竖线边框，无背景色区分。代码和对话文字视觉上非常接近，长代码段难以辨识边界。

**方案**: 代码块行用 `Span::styled` 设置 `bg = Color::Rgb(0x1a, 0x1b, 0x26)`（UI_DESIGN 已规划，仅需改 `draw_chat_area()` 中代码块渲染部分，~5 行）。对比度提升效果显著。

#### U3 ★★★★ 发送前确认（长消息场景）

**现状**: Ctrl+Enter 立即发送，无法撤销。误触时浪费 API 配额。对于代码审查场景（消息长达数百行），错误发送的损失更大。

**方案**: 增加一个可配置的"发送确认"开关（默认关闭）。开启后，输入超过 N 行的消息时，首次 Ctrl+Enter 弹出确认提示，再次 Ctrl+Enter 确认发送。

#### U4 ★★★★ 对话轮次分隔线

**现状**: 不同轮次的对话之间无视觉分隔，长对话中难以快速区分问答边界（UI_DESIGN 已规划未实现）。

**方案**: 每条消息前插入虚线分隔 `─ ─ ─ ─ ─ ─ ─ ─ ─`（颜色 `#3b4261`），在 `draw_chat_area()` 中非首条消息时追加。

#### U5 ★★★★ API 余额查询

**现状**: 用户无法从应用内获知 API 配额余额。对于按量付费场景，需要额外打开浏览器查看。

**方案**: 启动时对已知 provider 调用余额接口（DeepSeek `GET /user/balance`，MiMo 对应接口），标题栏 API 状态旁显示 `¥12.34` 余额。对不支持的 provider 则跳过。

#### U6 ★★★★ 会话导出 Markdown

**现状**: 有价值的对话无法导出分享。用户只能手动截图或复制粘贴（丢失语法高亮和格式）。

**方案**: 新增 `/export [path]` 命令，将会话输出为 `.md` 文件：
- YAML frontmatter 元信息（日期/模型/token/费用）
- 用户消息 `## User` + MiMo 回复 `## MiMo`
- 代码块保留语法高亮（嵌入 markdown 就是 ` ``` ` 围栏）

#### U7 ★★★ 输入框多行自动扩展

**现状**: 输入框固定 3 行。粘贴长代码段时需要外部编辑器写好再粘贴（还不能粘贴）。对于需要提供大段上下文的场景（'帮我重构这段代码'），输入体验极差。

**方案**: 输入内容超过当前可见区域时自动扩展输入框高度（最多到半屏），动态调整布局约束。

#### U8 ★★★ Markdown 粗体/斜体/列表渲染

**现状**: MiMo 回复中的 `**粗体**` / `*斜体*` / `- 列表项` 显示为纯文本，没有任何特殊渲染。对比 ChatGPT 终端的渲染效果，阅读体验差距明显。

**方案**: 在 `draw_chat_area()` 的消息渲染循环中，逐行检测行首模式并叠加 ratatui Style：
- `**text**` → `Modifier::BOLD`
- `*text*` → `Modifier::ITALIC`（终端支持时）
- `` `code` `` → 反色背景高亮
- `- ` / `* ` 开头 → `  • ` 前缀 + 缩进
- `1. ` 开头 → `  1. ` 前缀

#### U9 ★★★ 错误信息历史

**现状**: 错误信息仅在状态栏显示一次（下次按键即被替换为 copy_status）。用户想仔细阅读错误详情时已经消失了。

**方案**: `AppState` 新增 `error_history: Vec<String>`（最多 20 条），状态栏错误持续显示直到下一条新错误，同时提供 `/errors` 命令查看历史。

#### U10 ★★★ 终端窗口尺寸检查

**现状**: 窗口 < 60×20 时布局崩坏（标题栏文字重叠、状态栏截断、输入框变形），但无任何提示。UI_DESIGN 已规划未实现。

**方案**: 每次 draw 前检查 `f.area()` 尺寸，太小则绘制居中警告 `"窗口过小，请调整到 60×20 以上"`（~20 行）。

#### U11 ★★ 会话侧边栏

**现状**: F2 盲切会话，无法一览当前有哪些会话以及各自的消息数。UI_DESIGN 已设计完整方案。

**方案**: Ctrl+B 呼出左侧 24 字符侧边栏，显示会话列表 + 消息数 + 更新时间 + 新建按钮（~80 行）。

#### U12 ★★ 流式 token 计数精度优化

**现状**: Anthropic 流式解析器中 `output_tokens += 1` 假设每个 text_delta 恰好 1 个 token，实际上一个 delta 可能含多个 token。真正的准确值在 message_delta 的 usage 中。

**方案**: 去掉粗略累加（或仅在流式过程中做粗略显示并标注"~"），在 Done 时用 API 返回的精确值覆盖。

#### U13 ★★ 模型参数可配置（temperature / top_p）

**现状**: 配置文件仅支持 model 和 max_tokens，不支持 temperature、top_p 等推理参数。无法调整 MiMo 的输出风格（创造性/确定性）。

**方案**: Config 新增可选的 `temperature` / `top_p` 字段，构建请求时按配置传递（Anthropic 和 OpenAI 格式都支持这些参数）。

#### U14 ★ `check_api` 省配额优化

**现状**: 启动时发送 `"hi"` + `max_tokens=1` 探测 API。虽然 token 量极小，但可以更优雅——skip 启动检测，首条实际消息自然验证 API 可用性。

**方案**: 移除启动时的 `check_api()` 调用，首条消息发送失败时将错误信息展示在状态栏。或仅在启动时做一个无 body 的 HEAD 请求验证连通性。

#### U15 ★ 引用块视觉区分

**现状**: MiMo 回复中的 `> 引用文字` 显示为普通文本，无缩进、无竖线、无颜色区分。UI_DESIGN 配色规划了引用块边框色 `#3d59a1` 但未实现。

**方案**: `draw_chat_area()` 中识别 `>` 开头行，用竖线 + 缩进 + 特定颜色渲染，连续 `>` 行合并为一个引用块（~30 行）。

---

### 综合评分更新

| 维度 | v0.3.2 | v0.3.3 (建议) | 说明 |
|------|--------|---------------|------|
| 安全性 | 8.5/10 | 8.5/10 | P0 安全已全部到位，本次无新增安全问题 |
| 稳定性 | 7.5/10 | 8.0/10 | B1 SSE 兼容性修复可避免一类静默失败 |
| 代码质量 | 6.5/10 | 7.5/10 | 修复 26 clippy 警告 + B3/B5 代码缺陷 |
| 测试覆盖 | 0.0/10 | 0.0/10 | 仍为零测试，需尽快补充（见 T-1） |
| UI/UX | 7.0/10 | 7.5/10 | U1粘贴 + U2代码块背景 + U4分隔线 + U7搜索 |
| 功能完整度 | 7.5/10 | 7.5/10 | 核心功能完整，U5余额 + U6导出 为增值 |
| **总评** | **~7.7** | **~8.1** | 修复功能性 bug + 关键 UX 优化后目标 |

---

### 本次审查总结 (v0.3.3)

- **发现 Bug**: 9 个（P1×1, P2×4, P3×4），无 P0 安全漏洞
- **Clippy 警告**: 26 个，均为风格/最佳实践问题
- **用户优化建议**: 15 条（其中 U1 粘贴、U2 代码块背景、U4 分隔线性价比最高）
- **测试状态**: 0 tests（最大风险项）

### v0.3.4 修复总结

- **修复 Bug**: 8/9（B1-B5 已在之前版本修复，B6/B7/B8 本次修复）
- **新功能**: U1 Ctrl+V 粘贴 + U2 代码块深色背景
- **Clippy**: 27 → 0 warnings
- **评分**: 7.7 → 8.1+

---
---

## 项目快照

- **当前版本**：v0.3.5
- **当前行数**：~3661 行 Rust（11 个源文件）
- **依赖**：tokio, reqwest, serde, serde_json, ratatui, crossterm, futures-util, dirs, anyhow, scopeguard, unicode-width, syntect, cli-clipboard
- **编译状态**：通过，0 errors，0 clippy warnings
- **测试覆盖**：0 tests
- **API 端点**：4 provider 预设（MiMo Token Plan / DeepSeek / OpenAI / 自定义 OpenAI 兼容）
- **默认模型**：deepseek-chat（当前配置）
- **API 格式支持**：Anthropic（默认）/ OpenAI 兼容（DeepSeek 实测通过）
- **待完成条目**：0 P0 + 0 P1 + 9 P2 + 7 P3（U1-U4/U6/U9-U10/U14 已完成）
