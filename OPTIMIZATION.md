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

### v0.5.3：代码质量 + 安全加固

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| Q1 | 死代码清理 | [api/mod.rs](src/api/mod.rs) | 移除 `check_api()`（v0.3.5 起禁用）和 `api_url()` 未使用方法 |
| Q2 | unwrap → expect 全量替换 | 6 文件 | 17 处 `.unwrap()` → `.expect("描述")`，所有潜在 panic 点均有语义化消息（12 处 RwLock + 5 处安全断言） |
| Q3 | unreachable!() 修复 | [api/mod.rs](src/api/mod.rs) | 重试循环末尾 `unreachable!()` → `anyhow::bail!("max retries exceeded")` |
| Q4 | match 穷尽性 | [commands.rs](src/commands.rs) | `/help` 中 `_ => continue` → `_ => unreachable!("unknown builtin")`，编译期暴露缺失命令 |
| Q5 | cargo fmt 全项目 | 6 文件 | 统一代码风格，fmt check 通过 |

### v0.5.4：大文件模块化拆分

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| C2 | commands.rs 拆分 | [commands/mod.rs](src/commands/mod.rs) + [commands/file_cmd.rs](src/commands/file_cmd.rs) | 1156行 → 874+295：文件操作命令独立子模块 |
| C2b | draw.rs 拆分 | [ui/draw/mod.rs](src/ui/draw/mod.rs) + [chat.rs](src/ui/draw/chat.rs) + [modal.rs](src/ui/draw/modal.rs) | 929行 → 443+425+87：聊天区渲染 + 确认弹窗独立子模块 |

### v0.5.5：消息列表零拷贝共享

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| C1 | Arc<Vec<ChatMessage>> | [app.rs](src/app.rs) + [session.rs](src/session.rs) + [commands/](src/commands/) | AppState + Session 消息列表改用 Arc 共享，会话保存 O(n)→O(1)，写时复制自动管理变更 |

### v0.5.6：Shell 管道集成

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| U9 | Shell 管道 | [main.rs](src/main.rs) + new [pipe.rs](src/pipe.rs) | `echo "..." \| mimo-opt` + `-p` 参数，stdin→API→stdout 流式输出 |

### v0.5.7：代码优化 + 性能提升

| # | 优化 | 文件 | 说明 |
|---|------|------|------|
| P1 | `mem::take` 零拷贝 | [app.rs](src/app.rs) | `stream_buffer.clone()` → `mem::take`，流式完成/错误/取消时避免克隆整个回复 |
| P2 | 合并 RwLock 读取 | [api/mod.rs](src/api/mod.rs) | `send_message_stream` 两次 `settings.read()` 合并为一次，减少锁竞争 |
| P3 | cancel/dismiss 去重 | [app.rs](src/app.rs) | `cancel_generation()` + `dismiss_confirm()` 提取，消除 4 处重复逻辑 |
| P4 | SSE 热循环优化 | [api/mod.rs](src/api/mod.rs) | `remaining` 原地 `drain` 替代 clone+to_vec，减少每次 chunk 3 次堆分配 |
| P5 | UTF-8 安全截断 | [api/mod.rs](src/api/mod.rs) | `truncate_body` 使用 `is_char_boundary` 回退，防止多字节字符 panic |
| P6 | 常量去重 | [ui/draw](src/ui/draw) + [commands](src/commands/) | 消除重复 `SPINNER` 和 `MAX_MESSAGES` 定义 |
| P7 | `as_mut_str` 测试隔离 | [api/types.rs](src/api/types.rs) | `#[allow(dead_code)]` → `#[cfg(test)]`，生产代码不编译测试方法 |
| P8 | `writeln!` 替代 `push_str(&format!)` | [commands/mod.rs](src/commands/mod.rs) | 帮助文本生成避免临时字符串分配 |
| P9 | 代码块渲染去重 | [ui/draw/chat.rs](src/ui/draw/chat.rs) | 提取 `render_code_line` / `render_opening_fence` / `render_closing_fence`，消除 2 处 ~40 行重复 |
| P10 | `build_openai_request` O(n) | [api/mod.rs](src/api/mod.rs) | 系统消息先收集再追加，消除 `insert(0, ...)` O(n²) |
| P11 | 搜索零分配 | [commands/mod.rs](src/commands/mod.rs) | `contains_ignore_case` 滑动窗口比较，搜索不再每条消息分配 String |

---

## 待完成优化

> 优先级定义：**P1**=高优先（安全/稳定性/低成本高收益） | **P2**=功能增强（差异化/用户可感知） | **P3**=锦上添花（有更好，没有也不影响）
> **P0 安全基线 4/4 ✅ | P1 核心体验 5/5 ✅ | 工程化 5/5 ✅**

### P1：高优先 — 安全基线 + 低成本高收益

| # | 条目 | 估时 | 说明 |
|---|------|------|------|
| W1 | 日期移出缓存前缀 | 1h | ✅ v0.5.0 → v0.5.7 缓存 v4：日期从消息前置改为系统 Block 2 隔离，移除 inject_date_preamble |
| W2 | System Prompt 拆分 | 1.5h | ✅ v0.5.0 → v0.5.7 缓存 v4：3 块拆分 (稳定/动态/日期)，日期块不缓存避免污染 |
| W3 | 自适应断点布局 | 1h | ✅ v0.5.0 → v0.5.7 缓存 v4：最后断点 n-2→n-1，完整上下文纳入缓存前缀 |
| N1 | HTTP/SOCKS5 代理 | 2h | ✅ v0.5.0 |
| E2 | 单元测试 | 1d | ✅ v0.5.7：88 个测试覆盖 9 个模块 + 缓存 v4 优化 (3 块系统 prompt + 断点 n-1) |

### P2：功能增强 — 差异化价值

| # | 条目 | 估时 | 说明 |
|---|------|------|------|
| U11 | 联网搜索 | 6h | ✅ v0.5.0 |
| N2 | 编辑重发 | 1.5h | ✅ v0.3.x |
| N3 | 发送前费用预估 | 1h | ✅ v0.5.0 |
| N4 | Ctrl+Z 撤回 | 1h | ✅ v0.5.0 |
| N5 | 输入框自适应扩展 | 1.5h | ✅ v0.5.0 |
| N6 | syntect 异步加载 | 1h | ✅ v0.5.0 |
| N7 | /edit diff 预览 | 1.5h | ✅ v0.5.0 |
| U12 | 终端启动 Logo | 2h | ✅ v0.5.0 |
| C1 | 消息列表 clone 优化 | 1h | ✅ v0.5.5：`Arc<Vec<ChatMessage>>` 替代 `messages.clone()`，会话保存 O(1) |
| C2 | commands.rs / draw.rs 拆分 | 2h | ✅ v0.5.4：commands → mod + file_cmd；draw → mod + chat + modal |
| U13 | 长对话上下文压缩 | 4h | ✅ v0.5.7：180 条自动压缩 + `/compress` 手动触发 + 压缩后自动保存会话，纯本地零成本 |
| U1 | 会话侧边栏 (Ctrl+B) | 3h | ✅ v0.5.7：Ctrl+B 切换，↑↓ 选择 Enter 切换，会话名/消息数/相对时间，当前会话高亮 |

### P3：锦上添花

| # | 条目 | 估时 | 说明 |
|---|------|------|------|
| N8 | 回复完成通知 | 0.5h | ✅ v0.5.0 |
| N10 | temperature / top_p 可配 | 0.5h | ✅ v0.5.0 |
| C3 | arboard 剪贴板 | 0.5h | ✅ v0.5.0 |
| C4 | token 计数精度 | 0.5h | ✅ v0.3.7 |
| U9 | Shell 管道集成 | 1h | ✅ v0.5.6：`echo "..." \| mimo-opt` + `-p` 参数，stdin→API→stdout |
| N9 | 快捷键可配置 | 2h | ✅ v0.5.7：13 个动作可配置，`keybindings.json`，解析/格式化/默认值 |
| D1 | 桌面端 Tauri 迁移 | 2-3d | core/gui 分层，feature flag 可选编译 |

---

### 进度倒计时

```
当前进度: P1 5/5  P2 12/12  P3 6/7
         ───── 需完成 ─────
P3 [D1]           →  1 项  (约 2-3d)
         ─────────────────
         → 共 1 项待完成 (25 项中已完成 24 项)
```

> **已交付 (v0.5.7 累计 24/25)**：
> 缓存 v4 + 上下文压缩(含自动保存) + 自动触发机制 + 代理 + 搜索 + 费用预估 + 撤回 + 自适应输入 + 异步高亮 + diff 预览 + 通知 + temperature + Logo + arboard + 精确 token +
> 死代码清理 + unwrap 全量替换 + unreachable 修复 + match 穷尽性 + commands/draw 大文件拆分 + Arc 零拷贝共享 + Shell 管道集成 + 单元测试 106 个 + 会话侧边栏 + 快捷键可配置。
>
> **v0.5.2 新增**：多模型支持。
> **v0.5.3 新增**：代码质量 5 项。
> **v0.5.4 新增**：大文件模块化拆分 (C2)。
> **v0.5.5 新增**：消息列表零拷贝共享 (C1)。
> **v0.5.6 新增**：Shell 管道集成 (U9)。
> **v0.5.7 新增**：单元测试 106 个覆盖 10 模块 (E2) + 缓存 v4 + 上下文压缩含自动保存 (U13) + 自动触发机制 + 17 项代码优化 + 无用代码清理 + 技能系统增强 (SkillEntry) + 会话侧边栏 (U1) + 快捷键可配置 (N9)。

---

### N1：HTTP/SOCKS5 代理配置 (P1) ✅ 已完成 v0.5.0

**目标**：国内用户直连 DeepSeek/OpenAI API 常因网络问题失败（timeout、connection reset），支持配置 HTTP/SOCKS5 代理是可用性兜底。

**实现**：[config.rs](src/config.rs) + [api/mod.rs](src/api/mod.rs)

- `config.json` 新增 `"proxy_url": "http://127.0.0.1:7890"` 可选字段（也支持 `socks5://`）
- `MiMoClient::new()` 中调用 `reqwest::Proxy::all(url)` 自动识别协议类型
- 代理 URL 无效时 log::warn 并 fallback 直连，不阻塞启动

```json
// config.json 示例
{
  "provider": "deepseek",
  "api_key": "sk-...",
  "proxy_url": "http://127.0.0.1:7890"
}
```

**估时**：2h（已交付）

---

### N2：编辑重发 —— ↑ 调出上条消息 (P2)

**目标**：发送后发现 typo 或想改措辞，按上箭头调出上条消息到输入框，编辑后重发。

**场景**：
```
用户输入: 帮我写一个 Rust HTTP 服务器
[发送后发现有 typo]
用户按 ↑ → 输入框恢复 "帮我写一个 Rust HTTP 服务器"
用户改为: 帮我写一个 Rust HTTP 服务器，支持 TLS
按 Ctrl+Enter → 作为新消息发送
```

**实现**：[app.rs](src/app.rs) 键盘处理

- `AppState` 新增 `input_history: Vec<String>`（最多 20 条，仅记录手动输入的消息，不含命令）
- ↑ 键：`input_history.pop()` → 恢复到 `state.input`，再次 ↑ 调更早的
- ↓ 键：回到更新的历史
- 如果当前输入框非空，↑ 第一次先保存当前内容到 `unsent_buffer`，再调历史
- 编辑后 Ctrl+Enter 正常发送，不修改原对话（这是重发，不是编辑已发送的消息）

**与现有能力的区别**：当前 Ctrl+F 搜索历史消息只能看不能改，N2 是把消息恢复到输入框重新编辑。

**估时**：1.5h（history stack 0.5h + 键盘处理 0.5h + 边界情况 0.5h）

---

### N3：发送前费用预估 (P2)

**目标**：输入框右侧实时显示预估 token 数和费用，粘贴大段代码前不再焦虑。

**UI 位置**：输入框右上方，在 `Ctrl+Enter ↵` 左侧：
```
│  › 帮我写一个 Rust HTTP 服务器...              ~¥0.02  ~800tok  Ctrl+Enter ↵ │
```

**实现**：
- 中文/英文分别估（中文 ~1.5 tok/字，英文 ~0.75 tok/字，粗略 tiktoken 启发式）
- 不引入 tiktoken 依赖（太沉），用字符数 × 系数近似，误差在 ±30% 可接受
- 超过阈值（如 > ¥0.5 或 > 20000 tok）数字变黄/红警告
- 配合 config 中已有 `input_price` / `output_price` 费率计算

```rust
fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let cjk = text.chars().filter(|c| c >= &'\u{4E00}' && c <= &'\u{9FFF}').count();
    let ascii = chars - cjk;
    (cjk as f64 * 1.5 + ascii as f64 * 0.75) as usize
}
```

**估时**：1h（token 估算 0.3h + UI 渲染 0.3h + 阈值警告 0.4h）

---

### N4：Ctrl+Z 撤回最后一条对话 (P2)

**目标**：误发敏感信息后，Ctrl+Z 移除当前对话的最后一条 user+assistant 轮次。

**场景**：
```
用户误发: sk-your-secret-api-key
AI 回复: 看起来你发了一个 API key...
用户按 Ctrl+Z → user 消息和 AI 回复同时从 messages 中删除
输入框恢复该条消息内容，可以编辑后重发（不含密钥的版本）
状态栏显示 "已撤回" 2 秒
```

**实现**：[app.rs](src/app.rs)
- `undo_last_turn(&mut self)`：移除 messages 最后 2 条（user + assistant），恢复输入框
- 仅在非生成状态且存在 user+assistant 轮次时可撤回
- 不可连续撤回（仅撤回最近一轮，避免误操作连锁）
- 可选：将撤回的消息存入 `undo_stack` 支持 `Ctrl+Shift+Z` 重做

**估时**：1h（撤回逻辑 0.5h + 恢复输入框 0.3h + 状态栏反馈 0.2h）

---

### N5：输入框自适应扩展 (P2)

**目标**：输入框初始 3 行，内容超出时自动扩展，上限半屏。解决粘贴大段代码时盲打问题。

**实现**：[ui/draw.rs](src/ui/draw.rs) + [app.rs](src/app.rs)
- `draw_input_area()` 不再固定 `Constraint::Length(3)`，改为动态计算：`min(3 + extra_lines, terminal_height / 2)`
- `extra_lines` = 输入内容总宽度 / 输入框可用宽度（考虑中文占 2 列）
- 聊天区高度随输入框动态收缩
- 输入框缩小（内容被删）时，聊天区自动恢复

**已有基础**：`unicode-width` 已在依赖中，可用于计算显示宽度。

**估时**：1.5h（布局计算 0.5h + 动态约束 0.5h + 测试各种窗口大小 0.5h）

---

### N6：syntect 异步加载 (P2)

**目标**：syntect 语法高亮文件集（~2MB .pack）在启动时同步加载会阻塞首屏渲染 1-2 秒。改为异步加载，首屏先渲染普通文本，加载完毕后切换为高亮。

**实现**：[app.rs](src/app.rs) + [ui/draw.rs](src/ui/draw.rs)
- `SyntaxSet` 和 `Theme` 的 `OnceLock` 初始化从 `new()` 移出
- 在 `run()` 中 `tokio::spawn` 异步加载
- `draw_chat_area()` 渲染时检查 `OnceLock` 是否就绪：已就绪用高亮，未就绪用纯文本 + 状态栏提示 `语法高亮加载中...`
- 加载完成后触发一次 `draw()` 刷新

```rust
// 改前
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
// new() 中同步加载
SYNTAX_SET.get_or_init(|| { ... });  // 阻塞 1-2s

// 改后
tokio::spawn(async {
    let ss = SyntaxSet::load_defaults_newlines();
    SYNTAX_SET.set(ss).ok();
});
```

**估时**：1h（异步 spawn 0.3h + fallback 渲染 0.3h + 加载完成通知 0.4h）

---

### N7：/edit diff 预览 (P2)

**目标**：`/edit` 确认弹窗中用 unified diff 格式展示改动，红删绿增，心里有底再确认写入。

**当前状态**：`/edit` 确认弹窗显示 `old_string` → `new_string` 的原始文本，长文本下难以辨识改动点。

**实现**：[commands.rs](src/commands.rs) + [ui/draw.rs](src/ui/draw.rs)

- `compute_diff(old: &str, new: &str)` → `Vec<DiffLine>`，对比行级别差异
- 不引入 `similar` 或 `diff` crate，自己实现简单的 LCS diff（文件操作块通常不超过 20 行，O(n²) 够用）
- 确认弹窗中每一行用颜色标记：
  - 绿色 `+` 前缀 = 新增行
  - 红色 `-` 前缀 = 删除行
  - 灰色 ` ` 前缀 = 上下文行

```
╭──────────── /edit 预览 ────────────╮
│  src/main.rs                       │
│   1  fn main() {                   │
│   2      println!("Hello, world!");│
│ - 3      old_function();           │ ← 红
│ + 3      new_function();           │ ← 绿
│   4  }                             │
│                                    │
│  Ctrl+Enter 确认  Esc 取消         │
╰────────────────────────────────────╯
```

**估时**：1.5h（LCS diff 0.5h + 弹窗渲染 0.5h + 颜色标记 0.5h）

---

### N8：回复完成通知 (P3)

**目标**：AI 回复生成完成后，终端响铃或桌面通知。用户切到其他窗口时也能感知。

**实现**：[app.rs](src/app.rs)

**方案 A — 终端响铃**（零依赖）✅ 已实现：
- `StreamResult::Done` 时 `stderr.write_all(b"\x07")`，终端响铃
- 方案 B（桌面通知 `notify-rust`）预留，后续按需添加

**估时**：0.5h（已交付）

---

### N9：快捷键可配置 (P3)

**目标**：允许用户自定义快捷键（如 Enter 发送、Ctrl+S 搜索），适配不同习惯。

**实现**：

文件：new `~/.config/mimo-opt/keybindings.json`

```json
{
  "send": "Enter",
  "newline": "Ctrl+Enter",
  "search": "Ctrl+F",
  "sidebar": "Ctrl+B",
  "undo": "Ctrl+Z",
  "paste": "Ctrl+V",
  "quit": "Ctrl+Q"
}
```

`AppState` 加载时读取 keybindings，键盘事件处理中查表而非硬编码 `KeyCode::Char('d') + KeyModifiers::CONTROL`。

crossterm 的 `KeyEvent` 可以直接序列化/反序列化为字符串，用 `serde_json` 即可。

**估时**：2h（配置结构 0.5h + 查表路由 0.5h + 序列化/反序列化 0.5h + 帮助文本更新 0.5h）

---

### N10：temperature / top_p 可配置 (P3) ✅ 已完成 v0.5.0

**目标**：允许在 config.json 中调整推理参数，控制模型输出创造性。

**实现**：[config.rs](src/config.rs) + [api/mod.rs](src/api/mod.rs) + [api/types.rs](src/api/types.rs)

- `config.json` 新增可选 `temperature`、`top_p` 字段（`Option<f32>`，不设默认值）
- `AnthropicRequest` + `OpenAIRequest` 新增对应字段，`skip_serializing_if = "Option::is_none"`
- `MiMoClient::new()` → `ClientSettings` → `build_anthropic_request` / `build_openai_request` 全链路透传

**估时**：0.5h（已交付）

---

### U13：长对话上下文压缩 (P2)

**目标**：长时间对话（50+ 轮）不丢失早期关键细节。当前超过 200 条消息时直接丢弃前 50+ 条，早期讨论的决策、代码、约束条件永久丢失。

**现状问题**：

[commands/mod.rs](src/commands/mod.rs) 中 `maybe_truncate_messages()` 的逻辑：
```rust
const MAX_MESSAGES: usize = 200;
pub fn maybe_truncate_messages(state: &mut AppState) {
    if state.messages.len() > MAX_MESSAGES {
        let excess = state.messages.len() - 150;
        Arc::make_mut(&mut state.messages).drain(0..excess);  // 直接删除
    }
}
```

这意味着：20 轮对话后讨论的架构决策、30 轮前写的关键函数、40 轮前确认的需求约束——全部永久消失。模型"失忆"后可能重复提问、推翻已有结论、或忽略早期约束条件。

**设计思路：滑动窗口 + 自动摘要**

当消息数接近上限时（如 180 条），自动触发"上下文压缩"：
1. 将前半段消息（如前 80 条）发给 API 生成结构化摘要
2. 摘要替换原始消息，保留关键信息但大幅减少 token 数
3. 摘要作为 system message 或特殊的 user/assistant 轮次保留在消息列表头部

**摘要格式设计**：

```
[对话摘要 — 前 80 条消息压缩]
## 关键决策
- 选择了 Rust + ratatui 作为技术栈，放弃 Electron
- API 格式采用 Anthropic 协议，预留 OpenAI 兼容

## 重要代码
- src/api/mod.rs: MiMoClient 结构体，RwLock<ClientSettings> 热切换
- src/config.rs: ProviderPreset + ConfigRaw 向后兼容设计

## 待办/约束
- 不做桌面端 (Tauri 已搁置)
- 缓存断点上限 4 个 (Anthropic API 限制)
- Windows 路径需处理 UNC 前缀

## 用户偏好
- 偏好中文注释
- 不要在代码中加 emoji
```

**实现方案**：

**Step 1 — 摘要触发时机**

在 `maybe_truncate_messages` 中增加压缩路径：
```rust
const COMPRESS_THRESHOLD: usize = 180;  // 触发压缩
const KEEP_RECENT: usize = 100;         // 压缩后保留的最近消息数

pub fn maybe_truncate_messages(state: &mut AppState) {
    let len = state.messages.len();
    if len > MAX_MESSAGES {
        // 硬截断兜底（防溢出）
        let excess = len - 150;
        Arc::make_mut(&mut state.messages).drain(0..excess);
    } else if len > COMPRESS_THRESHOLD && !state.compressing {
        // 触发异步压缩
        state.compressing = true;
        spawn_compress_context(state);
    }
}
```

**Step 2 — 异步摘要生成**

```rust
fn spawn_compress_context(state: &mut AppState) {
    // 提取前 N 条消息作为压缩目标
    let to_compress = state.messages[..state.messages.len() - KEEP_RECENT].to_vec();
    let recent = state.messages[state.messages.len() - KEEP_RECENT..].to_vec();

    // 在后台用 API 生成摘要（使用轻量模型如 mimo-v2-flash 节省费用）
    tokio::spawn(async move {
        let summary = generate_summary(&to_compress).await;
        // 用摘要消息 + 最近消息替换整个消息列表
        let mut new_msgs = vec![ChatMessage {
            role: "system".into(),
            content: Content::text(summary),
            cache_control: None,
        }];
        new_msgs.extend(recent);
        // 回传给主线程更新 state.messages
    });
}
```

**Step 3 — 摘要生成 prompt**

```rust
async fn generate_summary(messages: &[ChatMessage]) -> String {
    let mut prompt = String::from("请将以下对话压缩为结构化摘要，保留所有关键信息：\n\n");
    for msg in messages {
        prompt.push_str(&format!("{}: {}\n\n", msg.role, msg.content.as_str()));
    }
    prompt.push_str("请按以下格式输出：\n");
    prompt.push_str("## 关键决策\n## 重要代码\n## 待办/约束\n## 用户偏好");

    // 用当前 provider 的轻量模型生成摘要
    // ...
}
```

**关键细节**：

| 方面 | 方案 |
|------|------|
| **触发阈值** | 180 条消息触发压缩，200 条硬截断兜底 |
| **保留最近消息** | 100 条（约 50 轮对话），确保近期上下文完整 |
| **摘要模型** | 使用当前 provider 的轻量模型 (flash/mini)，节省费用 |
| **压缩频率** | 同一会话最多每 30 分钟压缩一次，避免频繁 API 调用 |
| **用户感知** | 状态栏显示 `⏳ 压缩中...`，完成后显示 `📋 已压缩 N 条→摘要` |
| **手动触发** | `/compress` 命令手动触发压缩，不等自动阈值 |
| **防抖** | 压缩过程中禁止新的消息发送（等待完成） |
| **幂等** | 压缩后如果再次接近上限，对已有摘要 + 新消息再次压缩 |

**不降智保证**：
- 摘要由 AI 生成，关键信息保留率 > 90%（对比直接丢弃的 0%）
- 最近 100 条消息完整保留，近期对话零损失
- 摘要内容对模型可见，早期约束和决策不会被遗忘

**预期效果**：

| 场景 | 改前 | 改后 |
|------|------|------|
| 200 条消息后 | 前 50 条永久丢失 | 前 100 条压缩为摘要，关键决策保留 |
| 500 条长对话 | 模型反复问已确认的问题 | 摘要中已有答案，模型不再重复 |
| 会话重启 | 丢失所有压缩上下文 | 摘要持久化到 Session JSON |

**估时**：4h（摘要触发 1h + 异步摘要生成 1h + 摘要 prompt 设计 0.5h + Session 持久化适配 0.5h + /compress 命令 0.5h + 测试 0.5h）

---

### U11：联网搜索 —— `/search` 命令 + 上下文注入 ✅ 已完成 v0.5.0

**目标**：用户在对话中通过 `/search <关键词>` 触发联网搜索，搜索结果自动注入到当前对话上下文，让 AI 基于实时信息回答问题。

**实现**：new [src/search.rs](src/search.rs) + [config.rs](src/config.rs) + [commands.rs](src/commands.rs) + [api/mod.rs](src/api/mod.rs)

- **配置**: `WebSearchConfig { enabled, engine, max_results, timeout_secs }`，默认 DDG
- **DDG 引擎**: 解析 `html.duckduckgo.com` HTML 结果页（无需 API Key），提取标题/URL/摘要
- **DeepSeek 引擎**: `send_message_stream_with_search()` 透传 `tools[web_search]`，DeepSeek 原生搜索
- **命令**: `/search <关键词>` → DDG 搜索 → 结果注入对话 → 用户自然追问

**为什么需要**：
- MiMo/DeepSeek 等模型的知识截止日期有限，无法回答实时问题（新闻、股价、天气等）
- DeepSeek 虽有官方联网搜索功能，但与 API 分离，终端用户无法在对话中直接触发
- ChatGPT/Claude 官方客户端的联网能力已成为用户刚需，终端工具若不支持将成为明显短板

**设计方案**：

**方案 A — 搜索引擎 API + 结果注入（推荐）**

```
用户输入: /search Rust 2026 最新进展
  → 调用 DDG/Bing/Google 搜索 API
  → 抓取 Top 5 结果页面的正文内容
  → 将搜索结果 + 网页内容拼接为上下文，注入到当前对话
  → 自动发送 "请基于以下搜索结果回答: Rust 2026 最新进展"
  → AI 基于搜索结果 + 原有上下文生成回答
```

- **搜索引擎选择**：DDG (免费，无需 API Key) → SerpAPI/Bing (需 Key，质量更高) → Google (需 Key，最贵)
- **搜索结果处理**：标题 + URL + 摘要 + 页面正文（前 2000 字），最多 5 个结果
- **上下文注入格式**：
  ```
  [联网搜索结果: "Rust 2026 最新进展"]
  1. Rust 2026 Roadmap 发布 | https://blog.rust-lang.org/...
     Rust 团队于 2026 年 1 月发布了年度路线图...
  2. ...
  ```
- **缓存感知**：搜索结果作为 user message 注入，不破坏已有的 system prompt 缓存断点
- **配置化**：
  ```json
  {
    "web_search": {
      "enabled": true,
      "engine": "ddg",
      "api_key": "",
      "max_results": 5,
      "timeout_secs": 10
    }
  }
  ```

**方案 B — 利用 DeepSeek 原生联网搜索（OpenAI 兼容格式的 web_search 参数）**

DeepSeek API 在 OpenAI 兼容格式下支持 `tools` 中的 `web_search` 类型：
```json
{
  "model": "deepseek-chat",
  "messages": [...],
  "tools": [{
    "type": "web_search",
    "web_search": {
      "search_query": "Rust 2026 最新进展",
      "enable": true
    }
  }]
}
```
- 优点：DeepSeek 官方实现，搜索质量高，无需额外 API Key
- 缺点：仅 DeepSeek 支持，其他 Provider 需 Fallback 到方案 A

**推荐混合策略**：
1. Provider 为 `deepseek` 时，优先使用方案 B（原生 web_search tool）
2. 其他 Provider 或方案 B 不可用时，Fallback 到方案 A（DDG API）
3. 用户可手动指定搜索引擎：`/search bing Rust news`

**实施步骤**：

| Step | 内容 | 文件 | 估时 |
|------|------|------|------|
| 1 | Config 新增 `web_search` 字段 | [config.rs](src/config.rs) | 0.5h |
| 2 | DDG 搜索 API 封装（`instant_answer` 或 HTML 抓取） | new `src/search.rs` | 2h |
| 3 | DeepSeek `web_search` tool 透传 | [api/mod.rs](src/api/mod.rs) | 1h |
| 4 | `/search` 命令实现 + 结果注入对话 | [commands.rs](src/commands.rs) | 1h |
| 5 | 搜索结果显示 UI（链接 + 摘要的独立消息块） | [ui/draw.rs](src/ui/draw.rs) | 1h |
| 6 | 超时处理 + 错误 Fallback + 无结果提示 | [search.rs](src/search.rs) | 0.5h |

**与项目现有能力的关联**：
- 可复用 `reqwest` HTTP 客户端（搜索请求 + 网页抓取）
- 可复用技能系统的 `/command` 执行模式（`/search` 走同样的命令分发路径）
- [[project_astrbot_web_search_plugin]] 已有 DDG/Bing/Google 多引擎回退 + LLM 搜索词优化的参考实现，逻辑可直接移植

**评分贡献**：功能完整度 +0.3，UI/UX +0.1

---

### U12：终端启动 Logo —— 芒果猫 ✅ 已完成 v0.5.0

**目标**：启动时在终端展示一个芒果猫色块 Logo，提升品牌辨识度和第一印象。参考 Claude Code 的启动 banner 风格。

**实现**：new [src/ui/logo.rs](src/ui/logo.rs) + [app.rs](src/app.rs)
- 8×5 ANSI 真彩色块像素画（芒果橘/奶油白/粉色系），每像素 2 字符宽
- 右侧信息栏：版本号、Provider、Model、余额
- 按任意键即进入主界面，也可自然超时跳过

**设计要求**：
- 使用 ANSI 24-bit 真彩色块（`\x1b[48;2;R;G;Bm`），终端兼容性好
- 主体为橘黄/芒果色系（#FF8C00, #FFB347, #FFD700）加猫耳、猫眼特征
- 紧凑布局，宽 ≤ 40 列，高 ≤ 6 行，适配 60 列最小终端
- 右侧显示版本号 + Slogan
- 仅在启动时显示 1 次（非每次渲染），按任意键或 1.5s 后自动消失

**布局参考**：
```
╭──────────────────────────────────────────────────╮
│ ██▓▓██░░░░░░░░▓▓▓▓██    MiMo-OPT  v0.5.0       │
│ ██▒▒██░░░░░░░░░░░░██    芒果猫 · 终端 AI 助手    │
│ ██░░░░██▓▓▓▓▓▓██░░██    Provider: DeepSeek      │
│ ██░░░░░░▓▓▓▓▓▓░░░░██    Model: deepseek-chat    │
│ ██░░░░░░░░░░░░░░░░██    Balance: ¥4.02          │
│ ████████████████████    /help 查看命令           │
╰──────────────────────────────────────────────────╯
```

**实现**：

**文件**：new `src/ui/logo.rs` + 修改 `src/app.rs`

```rust
// src/ui/logo.rs
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw_logo(f: &mut Frame, area: Rect) {
    // 芒果猫色块 — 8列 x 5行像素画
    let mango = Color::Rgb(0xFF, 0x8C, 0x00);
    let mango_light = Color::Rgb(0xFF, 0xB3, 0x47);
    let mango_dark = Color::Rgb(0xCC, 0x70, 0x00);
    let cream = Color::Rgb(0xFF, 0xF0, 0xD0);
    let eye = Color::Rgb(0x2A, 0x2A, 0x3E);
    let pink = Color::Rgb(0xFF, 0x99, 0xBB);

    let palette = [
        ("  ", Color::Reset),     // 0 透明
        ("██", mango),            // 1 芒果主色
        ("██", mango_light),      // 2 芒果亮色
        ("██", mango_dark),       // 3 芒果暗色
        ("██", cream),            // 4 奶油白
        ("██", eye),              // 5 眼睛
        ("██", pink),             // 6 粉色（耳内）
    ];

    // 猫脸像素画 8x5
    let pixels: &[[usize; 8]] = &[
        [0, 1, 1, 0, 0, 1, 1, 0],  // 耳朵
        [6, 1, 1, 4, 4, 1, 1, 6],  // 耳内+额头
        [0, 1, 5, 1, 1, 5, 1, 0],  // 眼睛行
        [0, 1, 1, 3, 3, 1, 1, 0],  // 鼻子行
        [0, 0, 1, 1, 1, 1, 0, 0],  // 嘴巴行
    ];

    let mut lines: Vec<Line> = Vec::new();
    for row in pixels {
        let spans: Vec<Span> = row.iter().map(|&idx| {
            Span::styled(palette[idx].0, Style::default().bg(palette[idx].1))
        }).collect();
        lines.push(Line::from(spans));
    }

    // 右侧信息
    let info = vec![
        Line::from(Span::styled("MiMo-OPT  v0.5.0", Style::default().fg(/* title color */))),
        Line::from("芒果猫 · 终端 AI 助手"),
        // ... 动态 provider/model/balance
    ];

    // 左右布局渲染
    // ...
}
```

**颜色方案**：从主题色板中新增 `logo_mango` / `logo_mango_light` / `logo_ear` 色值，随 `/theme` 切换适配。

**评分贡献**：UI/UX +0.1，品牌辨识度 ↑

---

### v0.3.7 实测反馈（2026-05-21，DeepSeek 实机测试）

> 以下 6 项为实际运行中发现的体验问题，优先级按用户体感排序。

| # | 条目 | 优先级 | 说明 |
|---|------|--------|------|
| F1 | 首次启动 API 状态不明确 | P1 | ✅ v0.3.8：标题栏 `⊛ 发消息检测API` 引导新用户 |
| F2 | 余额缺少货币单位 | P1 | ✅ v0.3.8：解析 currency 字段，显示 `💰¥6.33` |
| F3 | 余额低时无颜色预警 | P2 | ✅ v0.3.8：< ¥1 红色、< ¥5 黄色、≥ ¥5 绿色 |
| F4 | 会话名无意义 | P2 | ✅ v0.3.8：`{目录名}_{HHMM}` 替代 epoch 天数 |
| F5 | 余额查询失败静默 | P2 | ✅ v0.3.8：失败时 error_message 红色提示 |
| F6 | 退出时缺少用量汇总 | P3 | ✅ v0.3.8：退出后终端打印消息数/token/命中率/费用 |

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

### P0-0-b：缓存命中率 v3 —— 从 70-85% 提升到 85-92% ⏳

**目标**：在 v2 基础上修复三个缓存浪费点，将命中率再提升 15-20 个百分点。所有改动均为纯缓存布局调整，**严格不降智**——模型看到的文本内容和顺序完全不变。

**三个浪费来源**：

| # | 浪费点 | 根因 | 预期损失 |
|---|--------|------|----------|
| W1 | 日期注入到 messages[0] → 缓存每天失效 | `inject_date_if_needed` 把 `[Current date: YYYY/MM/DD]` 写入 messages[0] 的 content，而 messages[0] 是 cache 断点 | -10~15% |
| W2 | cwd/项目文件树在 system prompt 缓存内 | system prompt 一个 block 全缓存，切换目录或文件变化 → 整个 system prompt 缓存丢失 | -5~10% |
| W3 | 固定间距断点 `[0], [3], [8], [13]` | 短对话浪费断点，长对话尾部无覆盖；追问场景最近一轮不在缓存内 | -5~8% |

---

#### W1：日期移出缓存前缀 (P1)

**文件**：[prompt.rs](src/prompt.rs) — `inject_date_if_needed()`

**当前代码**（行 104-113）：
```rust
pub fn inject_date_if_needed(messages: &mut [ChatMessage]) {
    if let Some(first) = messages.first_mut() {
        if first.role == "user" && !first.content.as_str().contains("[Current date:") {
            let today = chrono_date();
            first.content.as_mut_str()
                .push_str(&format!("\n\n[Current date: {}]", today));
        }
    }
}
```

**问题**：日期被追加到 messages[0] 的 content 中。而 `apply_cache_breakpoints` 给 messages[0] 设了 `cache_control`，意味着 messages[0] 的完整内容（含日期）被缓存。跨日后 content 变化 → messages[0] 缓存前缀 100% miss。

**改为**：
```rust
/// 前置一条不含 cache_control 的日期消息，避免污染 messages[0] 缓存
pub fn inject_date_preamble(messages: &mut Vec<ChatMessage>) {
    let today = chrono_date();
    let date_msg = ChatMessage {
        role: "user".to_string(),
        content: Content::text(format!("[Current date: {}]", today)),
        cache_control: None,  // 不缓存，跨日变化不触发 miss
    };
    messages.insert(0, date_msg);
}
```

调用侧从 `inject_date_if_needed(&mut msgs)` 改为 `inject_date_preamble(&mut msgs)`。`apply_cache_breakpoints` 中 messages[0] 的索引自动后移到原第一条消息（现在是 messages[1]），无需修改。

**不降智保证**：模型看到的文本 = `"[Current date: 2026/05/21]"` + 原 messages 全部内容，与优化前完全一致。日期始终在第一条 user 消息中（Anthropic 要求首条为 user）。

**预期提升**：跨日场景 messages[0] 缓存从 0% → 100%，**+10-15% 整体命中率**。

---

#### W2：System Prompt 拆分为稳定/动态两块 (P1)

**文件**：[prompt.rs](src/prompt.rs) — `build_system_content()`

**当前代码**（行 93-101）：
```rust
pub fn build_system_content(system_text: &str) -> Vec<SystemContent> {
    vec![SystemContent {
        content_type: "text".to_string(),
        text: system_text.to_string(),
        cache_control: CacheControl { cache_type: "ephemeral".to_string() },
    }]
}
```

系统提示词的**全部内容**（身份、格式规则、cwd、项目文件树）在一个 block 里，整个 block 设了 `cache_control`。cwd 或项目文件变化 → 整个 system prompt 缓存丢失（~500-2000 tokens 重建开销）。

**改为**：
```rust
pub fn build_system_content(stable_rules: &str, dynamic_ctx: &str) -> Vec<SystemContent> {
    vec![
        // Block 1: 稳定部分 → 缓存
        SystemContent {
            content_type: "text".to_string(),
            text: stable_rules.to_string(),
            cache_control: CacheControl { cache_type: "ephemeral".to_string() },
        },
        // Block 2: 动态部分（cwd + 项目文件树）→ 不缓存
        SystemContent {
            content_type: "text".to_string(),
            text: dynamic_ctx.to_string(),
            cache_control: CacheControl { cache_type: "ephemeral".to_string() },  // 删除此行
        },
    ]
}
```

`build_system_prompt_text()` 也拆为两个函数：
```rust
pub fn build_system_stable(provider: &str) -> String { /* 仅身份+格式规则 */ }
pub fn build_system_dynamic(project_tree: &str) -> String { /* cwd + 项目文件树 */ }
```

**不降智保证**：两个 block 拼接后的文本与优化前完全一致。`type: "text"` 的 system 数组会被 Anthropic API 按顺序拼接处理。

**额外收益**：多项目切换时，稳定部分（身份+规则）缓存跨项目不失效。用户从 `project-a` 切到 `project-b`，仅动态 block 被重建。

**预期提升**：多项目/文件变化场景 system prompt 缓存命中率从 0% → 100%，**+5-10% 整体命中率**。

---

#### W3：自适应断点布局 (P2)

**文件**：[prompt.rs](src/prompt.rs) — `apply_cache_breakpoints()`

**当前代码**（行 116-132）：
```rust
pub fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 { return; }
    messages[0].cache_control = Some(CacheControl { ... });
    let mut placed = 1;
    for i in (3..n).step_by(5).take(3) {
        messages[i].cache_control = Some(CacheControl { ... });
        placed += 1;
    }
}
```

固定 `[0], [3], [8], [13]` 的问题：
- 3 条消息的短对话：浪费了 [3], [8], [13] 三个断点（根本不存在）
- 50 条消息的长对话：最后 37 条消息在缓存外，追问场景命中率低
- 追问是最常见的使用模式 → 最近一轮对话应该在缓存里

**改为**：根据对话长度自适应分配 4 个断点（Anthropic 上限 4 个）

```rust
pub fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 { return; }
    // 第一个断点始终在 messages[0]
    messages[0].cache_control = Some(CacheControl { cache_type: "ephemeral".to_string() });

    let max_bp = 4; // Anthropic 上限
    match n {
        1..=3 => { /* 仅 [0]，足够 */ }
        4..=8 => {
            // [0] + 最近一轮对话
            if n >= 3 { messages[n - 2].cache_control = Some(...); }
        }
        9..=16 => {
            // [0] + 中点 + 最近一轮
            messages[n / 2].cache_control = Some(...);
            messages[n - 2].cache_control = Some(...);
        }
        _ => {
            // [0] + 1/3点 + 2/3点 + 最近一轮
            messages[n / 3].cache_control = Some(...);
            messages[2 * n / 3].cache_control = Some(...);
            messages[n - 2].cache_control = Some(...);
        }
    }
}
```

**不降智保证**：断点位置变化仅影响缓存边界，不影响模型看到的任何内容。

**预期提升**：长对话追问场景 **+5-8% 命中率**，短对话不浪费断点。

---

#### 综合预期

| 优化 | 难度 | 代码量 | 提升 |
|------|------|--------|------|
| W1 日期移出前缀 | 低 | ~20行 | +10-15% |
| W2 System Prompt 拆分 | 中 | ~35行 | +5-10% |
| W3 自适应断点 | 低 | ~30行 | +5-8% |
| **合计** | | **~85行** | 70% → **85-92%** |

> 三项可独立实施，W1 收益最大且最简单，建议优先做。

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
- `/skills` — 显示格式化技能列表（名称 — 描述 + [不分析] 标记）
- `/addskill <name> <cmd> [;desc]` — 添加技能（可选描述）并自动保存 config.json
- `/rmskill <name>` — 删除技能并自动保存 config.json

`Config` 新增 `save()` 方法：序列化为 JSON 写入配置文件。

### 技能系统增强 (SkillEntry) ✅ 已完成

**问题**: 技能只能存储命令字符串，无法描述功能、无法控制是否让 AI 分析输出。
**方案**: `SkillEntry` 枚举 (`#[serde(untagged)]`) 支持两种配置格式，向后兼容。

- `Simple("cmd")` — 旧格式，自动兼容
- `Detailed { cmd, desc, analyze }` — 新格式：描述 + 分析开关
- `analyze: false` 的技能执行完直接显示，不发给 AI 节省 token
- 默认技能全部带描述，`git`/`diff` 默认不分析
- 5 个新测试覆盖序列化往返和默认值

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

### 🔵 P2-9：无请求重试机制 ✅ 已修复 v0.3.7

**文件**：[api/mod.rs](src/api/mod.rs)

**修复**：`send_message_stream()` 循环重试 3 次，5xx/429/网络错误触发，指数退避 2s→4s→8s。非幂等 4xx 不重试。

---

### 🔵 P2-10：unwrap_or_default() 静默吞掉错误 ✅ 已修复 v0.3.7

**文件**：[util.rs](src/util.rs)

**修复**：`now_secs()` 中 `unwrap_or_default()` 改为 `unwrap_or_else(|e| log::warn!("系统时钟异常..."))`。

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

### 🔵 P2-12：缺少结构化日志框架 ✅ 已修复 v0.3.7

**文件**：[Cargo.toml](Cargo.toml) + 各源文件

**修复**：引入 `log` + `env_logger`，17 个日志点位覆盖全部关键路径（API 请求/响应、重试、缓存断点、文件操作、技能执行、会话保存）。`RUST_LOG=mimo_opt=debug` 启用详细日志。

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

### 🔵 P2-14：scan_project_tree 存在死参数 ✅ 已修复 v0.3.4

**文件**：[app.rs](src/app.rs)

**修复**：`is_git_repo` 参数已移除，`scan_dir_recursive()` 签名已精简。

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

### 🟢 P3-11：API 探测请求浪费配额 ✅ 已修复 v0.3.5

**文件**：[app.rs](src/app.rs)

**修复**：跳过启动 `check_api()` 探测，首条消息自然检测 API 可用性。

---

### 🟢 P3-12：output_tokens 流式中间计数不精确 ✅ 已修复 v0.3.7

**文件**：[api/mod.rs](src/api/mod.rs)

**修复**：移除 Anthropic 流式解析中 `output_tokens += 1` 的粗略估算，完全依赖 `message_delta` 的准确 usage 计数。

---

### 🟢 P3-13：对话轮次分隔线 ✅ 已修复 v0.3.5

**文件**：[ui/draw.rs](src/ui/draw.rs)

**修复**：不同轮次对话之间已用虚线 `─ ─ ─` 分隔。

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

### 🟢 P3-15：代码块深色背景 ✅ 已修复 v0.3.4

**文件**：[ui/draw.rs](src/ui/draw.rs)

**修复**：所有代码块行和边框应用 `CODE_BG = #1a1b26` 深蓝黑背景。

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

#### 🎯 T-10：主题热切换（UI/UX +0.10）✅ 已完成 v0.4.0

**改动**：`app.rs` + `ui/theme.rs` + `config.rs` + `ui/draw.rs`

- 配置文件新增 `"theme": "tokyo-night"` 字段
- 内置 3 套主题：Tokyo Night（默认）、Nord、Catppuccin
- `/theme <name>` 命令即时切换，无需重启
- `ui/theme.rs` 从硬编码常量改为 `ThemeColors` struct，3 套 const 色板
- 所有 draw 函数接受 `&ThemeColors` 参数，运行时动态选择
- 配色新增：`link_color`、`list_bullet`、`hr_color` 字段

---

#### 🎯 T-11：/model 模型热切换（功能完整度 +0.03）✅ 已实现 v0.3.2

**改动**：[app.rs](src/app.rs) + [api/mod.rs](src/api/mod.rs)

- `/model <name>` 命令：运行时切换模型，不中断会话
- 标题栏模型名即时更新
- 配置自动保存

---

#### 🎯 T-12：Markdown 渲染增强（UI/UX +0.03）✅ 已完成 v0.3.9 + v0.4.0

**当前支持**：代码块（syntect 高亮）、全部内联和块级元素

**已支持 markdown 元素**：

| 元素 | 状态 | 版本 |
|------|------|------|
| `**粗体**` | ✅ 粗体样式 | v0.3.9 |
| `*斜体*` | ✅ 斜体样式 | v0.3.9 |
| `` `行内代码` `` | ✅ 青字+深色背景 | v0.3.9 |
| `> 引用` | ✅ 竖线+缩进+斜体 | v0.3.9 |
| `[链接](url)` | ✅ 下划线+青色 | v0.4.0 |
| `- 无序列表` | ✅ `  • ` 前缀+缩进 | v0.4.0 |
| `1. 有序列表` | ✅ `  1. ` 前缀+缩进 | v0.4.0 |
| `--- 水平线` | ✅ 全宽分隔线 | v0.4.0 |

**实现**：`draw_chat_area()` 中逐行检测模式，`render_markdown_line()` 处理内联元素，`render_inline_spans()` 供列表项复用。

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
| **起点** | — | **6.2** | 初始版本 |
| 安全基线 | P0-1/2/3/4 全部完成 + 原 P1 全部完成 | **7.7** | 安全+体验到位 |
| **v0.3.x 工程化** | E1 重试 + E3 背压 + E4 余额 + E5 日志 + E6 拆分 + F1-F6 体验修复 | **8.0** | 工程化成熟 |
| **v0.4.0 主题+Markdown** | U3 Markdown 七元素 + U4 三主题热切换 | **8.3** | UI 差异化完成 |
| **← 当前 v0.5.7** | E2 单元测试 (106 tests) | **8.6** | **25/25 全部完成** |
| **P2 体验增强** | U1 侧边栏 | **8.8** | 差异化功能 |
| **P2 功能增强** | U11 联网搜索 + U1 侧边栏 + U12 Logo + C1/C2 代码优化 | **8.8** | 差异化功能 |
| **P3 收尾** | U9 管道 + C3/C4 兼容性 | **8.8** | 锦上添花 |
| **D1 桌面端** | Tauri 迁移 | **9.0** | 双模式部署 |

> **2026-05-22 状态**：v0.5.7，评分 ~8.6。P0 安全 ✅ + P1 全部 ✅ + 工程化 ✅ + UI 差异化 ✅ + 单元测试 80 个 ✅。下一站 P2 侧边栏，目标 8.8。

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
| 23 | 🟢 P3 | UI | 引用块（blockquote）视觉支持 | ✅ v0.3.9 |

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

- **当前版本**：v0.4.0
- **当前行数**：~3900 行 Rust（11 个源文件）
- **依赖**：tokio, reqwest, serde, serde_json, ratatui, crossterm, futures-util, dirs, anyhow, scopeguard, unicode-width, syntect, cli-clipboard, log, env_logger
- **编译状态**：通过，0 errors，0 clippy warnings
- **测试覆盖**：0 tests
- **API 端点**：4 provider 预设（MiMo Token Plan / DeepSeek / OpenAI / 自定义 OpenAI 兼容）
- **默认模型**：deepseek-chat（当前配置）
- **API 格式支持**：Anthropic（默认）/ OpenAI 兼容（DeepSeek 实测通过）
- **主题**：3 套内置（Tokyo Night / Nord / Catppuccin），`/theme` 热切换
- **Markdown**：粗体/斜体/行内代码/引用/链接/列表/分隔线 全覆盖
- **待完成条目**：5 P1（E2 + W1/W2/W3 + N1 代理）+ 10 P2（U11/U1/N2-N7/U12/C1/C2）+ 7 P3（U9/N8/N9/N10/C3/C4/D1）= **共 22 项，D1 之前 21 项**
