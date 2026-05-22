# MiMo-OPT 更新日志

## v0.5.7 (2026-05-22) — 单元测试 + 缓存 v4 + 上下文压缩 + 代码优化 + 侧边栏 + 快捷键

**目标**: 补齐最大短板——零测试覆盖。106 个单元测试覆盖 10 个模块，缓存命中率优化 (不降智)，长对话上下文压缩（含自动保存），17 项代码优化 + 无用代码清理 + 技能系统增强 + 会话侧边栏 + 快捷键可配置。**25/25 全部完成。**

### 测试覆盖 (0 → 106 tests)

| 模块 | 测试数 | 覆盖内容 |
|------|--------|---------|
| `util` | 14 | estimate_tokens (空/ASCII/CJK/混合/日文)、format_cost (高/低/零)、now_secs、get_cwd、format_age (刚刚/分钟/小时/天) |
| `config` | 17 | mask_key (空/短/正常/边界)、find_preset、default_model、find_model、pricing、WebSearchConfig 默认值、Config Debug 脱敏、SkillEntry (5 项) |
| `keybindings` | 9 | parse (单字符/F键/Shift+Enter/大小写/特殊键/非法)、format_combo、defaults 匹配、describe 输出 |
| `prompt` | 12 | is_leap、chrono_date 格式、cache_breakpoints (空/单/双/中/长/上限4)、build_system_stable (3 provider)、content_split (3 block) |
| `file_ops` | 10 | reject_absolute_path、reject_traversal、reject_dot_git (2)、strip_unc_prefix (2)、format_lines (基本/偏移)、allow_normal_path |
| `api/types` | 11 | Content 序列化往返、ChatMessage (有/无 cache_control)、Usage (Anthropic/OpenAI 双格式)、SystemContent (Optional cache_control)、OpenAIRequest 序列化 |
| `session` | 5 | new 字段验证、id 时间戳格式、auto_name 格式、序列化往返、token 字段 default |
| `search` | 12 | url_escape、url_decode (基本/非法)、strip_html (基本/嵌套)、clean_ddg_url (3 种)、format_results (空/单/多)、parse_ddg_html 空 |
| `ui/theme` | 6 | get_theme (3 主题 + 未知回退)、theme_names、全部主题字段一致性 |
| `commands` | 10 | summarize_messages (空/用户/代码语言/截断/上限)、contains_ignore_case (基本/空/多字节/末尾) |

### 长对话上下文压缩 (U13)

**问题**: 超过 200 条消息时直接丢弃早期对话，关键上下文永久丢失。
**方案**: 达到 180 条时自动压缩——保留最近 100 条，旧消息合成结构化摘要（系统消息）。

- 自动触发：消息 ≥ 180 条时压缩 + 自动保存会话，不再硬截断丢弃
- 手动触发：`/compress` 命令（保留后 50%，前 50% 压缩为摘要）+ 自动保存
- 摘要内容：用户请求摘要、涉及主题/编程语言、关键回复要点
- 无 API 成本：纯本地文本提取，零延迟
- 压缩后自动保存会话，摘要持久化到磁盘

### 自动触发机制

| 条件 | 自动行为 |
|------|---------|
| 消息 ≥ 180 条 | 压缩上下文 + 保存会话 |
| `/compress` 手动触发 | 压缩 + 保存 + 反馈压缩结果 |
| 技能执行完 | 根据 `analyze` 标记决定是否发给 AI 分析 |
| 搜索结果返回 | 自动发给 AI 总结 |
| 输入时 | 实时显示命令补全提示 |
| 出错后 | 下次发送时自动清除错误 |

### 技能系统增强 (SkillEntry)

**问题**: 技能只能存储命令字符串，无法描述功能、无法控制是否让 AI 分析输出。
**方案**: `SkillEntry` 枚举支持两种配置格式（向后兼容），新增描述和分析控制。

- `SkillEntry::Simple("cmd")` — 旧格式，自动兼容
- `SkillEntry::Detailed { cmd, desc, analyze }` — 新格式，支持描述和分析开关
- `/addskill` 支持可选描述: `/addskill lint cargo clippy 2>&1 ;Rust 代码检查`
- `/skills` 显示描述 + `[不分析]` 标记
- `analyze: false` 的技能执行完直接显示结果，不消耗 AI token
- 默认技能全部带描述，`git`/`diff` 默认不分析（纯查看类）
- 配置文件自动兼容旧格式（string → Detailed），零迁移成本
- 测试: 5 个新测试 (SkillEntry 序列化往返、Simple 解析、默认技能描述)

### 会话侧边栏 (U1)

**问题**: 切换会话只能 F2 循环，无法看到会话列表和选择。
**方案**: Ctrl+B 打开左侧 24 列侧边栏，显示会话列表。

- Ctrl+B 切换侧边栏，显示会话名称、消息数、相对时间（刚刚/分钟前/小时前/天前）
- ↑↓ 导航选择，Enter 切换到选中会话，Esc 关闭
- 当前会话加粗高亮，选中项 `▸` 标记
- 终端宽度 < 40 列时自动隐藏侧边栏
- 新增 `SessionInfo.name` / `msg_count` 字段，`format_age()` 时间格式化
- 新增 `ui/draw/sidebar.rs` 模块，4 个 format_age 测试

### 快捷键可配置 (N9)

**问题**: 快捷键硬编码，无法根据用户习惯调整。
**方案**: `keybindings.json` 配置文件，13 个动作可自定义。

- 支持的动作: toggle_sidebar / new_session / next_session / send / quit / cancel / paste / search / copy_code / undo / cursor_home / cursor_end / scroll_top
- 解析格式: `"Ctrl+B"` / `"F2"` / `"Shift+Enter"` 等，大小写不敏感
- 文件不存在时使用默认值，只覆盖配置了的动作
- 新增 `keybindings.rs` 模块，`KeyBindings::load()` + `is()` + `describe()`
- 事件循环中 13 处硬编码替换为 `state.keybindings.is("action", ...)`
- 9 个测试覆盖解析、格式化、默认值、非法输入
- 配置路径: `%APPDATA%\mimo-opt\keybindings.json`

### 缓存 v4 优化 (不降智增加缓存命中率)

**系统 Prompt 3 块拆分**:
- Block 0 (稳定, 缓存): Provider 身份 + 输出格式规则
- Block 1 (动态, 缓存): CWD + 项目文件树
- Block 2 (日期, **不缓存**): 当前日期 — 从动态块中隔离，避免日期变化污染 CWD/文件树缓存

**断点优化**:
- 最后断点 `n-2` → `n-1`: 最后一条 assistant 回复纳入缓存前缀，下一轮对话可复用完整上下文
- `SystemContent.cache_control` 改为 `Option<CacheControl>`，日期块无需 `cache_control` 字段

**移除 `inject_date_preamble`**: 日期从消息前置改为系统 Block 2，不再污染消息缓存前缀

### 代码优化 (15 项)

| # | 优化 | 文件 | 效果 |
|---|------|------|------|
| 1 | `format!("{}", ts)` → `ts.to_string()` | session.rs | 消除冗余 format 调用 |
| 2 | `format_lines_with_numbers` 提取 | file_ops.rs | read_file 行号格式化 Some/None 分支去重 |
| 3 | `mem::take` 替代 `stream_buffer.clone()` | app.rs | 流式完成/错误/取消时零拷贝移入消息 |
| 4 | 合并双重 RwLock 读取 | api/mod.rs | `send_message_stream` 两次 read → 一次 |
| 5 | `cancel_generation()` 提取 | app.rs | Ctrl+C / Esc 取消生成 2 处 → 1 个函数 |
| 6 | `dismiss_confirm()` 提取 | app.rs | 'n' / Esc 取消确认 2 处 → 1 个函数 |
| 7 | SSE 热循环减少分配 | api/mod.rs | `remaining` 原地 `drain`，减少 3 次堆分配/chunk |
| 8 | `truncate_body` UTF-8 安全 | api/mod.rs | `is_char_boundary` 回退，防止多字节 panic |
| 9 | 常量去重 | ui/draw + commands | 消除重复 `SPINNER` 和 `MAX_MESSAGES` |
| 10 | `as_mut_str` → `#[cfg(test)]` | api/types.rs | 生产代码不编译测试专用方法 |
| 11 | `format!` → `writeln!` | commands/mod.rs | 帮助文本避免临时字符串分配 |
| 12 | 代码块渲染去重 | ui/draw/chat.rs | `render_code_line` / `render_opening_fence` / `render_closing_fence` 提取 |
| 13 | `build_openai_request` O(n) | api/mod.rs | 先收集 system 再收集 user，消除 `insert(0)` |
| 14 | `contains_ignore_case` 零分配 | commands/mod.rs | 滑动窗口比较，搜索不再每条消息分配 String |
| 15 | `inject_date_preamble` 移除 | prompt.rs + commands | 日期移入系统 Block 2，消除消息层冗余 |
| 16 | 无用代码清理 | 5 文件 | 移除 `system_prompt_text` 死字段、`build_system_prompt_text` 死函数、`SessionInfo` 死字段、14 个函数 `pub` 收窄 |
| 17 | 上下文压缩 | commands/mod.rs | `maybe_truncate_messages` 升级：180 条触发压缩而非 200 条截断丢弃 |

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/util.rs` | +56 行: 10 个测试 |
| `src/config.rs` | +76 行: 12 个测试 |
| `src/prompt.rs` | +80 行: 12 个测试 + 3 块系统 prompt + 断点 n-1 + 移除 inject_date_preamble + 删除 `build_system_prompt_text` |
| `src/file_ops.rs` | +62 行: 10 个测试 + `format_lines_with_numbers` 提取 |
| `src/api/types.rs` | +80 行: 11 个测试 + `Option<CacheControl>` + `#[cfg(test)]` gate |
| `src/api/mod.rs` | SSE 热循环 + RwLock 合并 + UTF-8 截断 + OpenAI 请求 O(n) |
| `src/app.rs` | `mem::take` + `cancel_generation()` + `dismiss_confirm()` + 删除 `system_prompt_text` 死字段 |
| `src/ui/draw/mod.rs` | `SPINNER` → `pub(super)` |
| `src/ui/draw/chat.rs` | 去除重复 SPINNER + 代码块渲染 3 函数提取 + `pub` → `fn` |
| `src/session.rs` | +42 行: 5 个测试 + 删除 `SessionInfo` 死字段 |
| `src/search.rs` | +67 行: 12 个测试 |
| `src/ui/theme.rs` | +51 行: 6 个测试 |
| `src/commands/mod.rs` | 上下文压缩 + `/compress` 命令 + 10 个测试 + `contains_ignore_case` + 死代码清理 + `pub` → `pub(crate)`/`fn` |
| `src/commands/file_cmd.rs` | `compute_edit_diff` → `fn` |
| `Cargo.toml` | 版本 0.5.6 → 0.5.7 |
| `CHANGELOG.md` | v0.5.7 条目 |
| `OPTIMIZATION.md` | E2 + 缓存 v4 + 代码优化 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy + fmt clean)
- **106 tests passed**

### 路线图更新

- **新增 U13**: 长对话上下文压缩 (P2, 估时 4h) — 接近消息上限时自动摘要旧消息，保留关键决策/代码/约束，替代当前的直接丢弃
- **总进度**: 21/25 完成 (P1 全满, P2 10/12, P3 5/7)

---

## v0.5.6 (2026-05-22) — Shell 管道集成

**目标**: 支持 `echo "..." | mimo-opt` 非交互式管道模式，将 MiMo-OPT 融入 shell 工作流。

### 新功能

- **管道模式**: `echo "请解释这段代码" | mimo-opt` — 从 stdin 读取输入，流式输出 API 响应
- **`--prompt` / `-p` 参数**: `mimo-opt -p "Rust 生命周期是什么"` — 直接传参非交互式问答
- **CLI 参数解析**: 轻量 `std::env::args()` 解析，零依赖新增
- **错误隔离**: 管道模式下 API Key 未配置时输出英文错误到 stderr，不阻塞脚本

### 使用示例

```bash
# 管道模式：将命令输出发给 AI 分析
cargo clippy 2>&1 | mimo-opt

# 参数模式：单次问答
mimo-opt -p "解释 Rust 的所有权系统"

# Shell 脚本集成
git diff | mimo-opt -p "review these changes"
```

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/main.rs` | CLI 参数解析 + 管道/交互模式分发 + `print_first_run_guide()` 提取 |
| `src/pipe.rs` | 新建: 管道模式实现（stdin 读取 → API 流式请求 → stdout 输出） |
| `Cargo.toml` | 版本 0.5.5 → 0.5.6 |
| `CHANGELOG.md` | v0.5.6 条目 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy + fmt clean)

---

## v0.5.5 (2026-05-22) — 消息列表零拷贝共享

**目标**: `AppState.messages` 和 `Session.messages` 改用 `Arc<Vec<ChatMessage>>`，消除会话保存和切换时的 O(n) 全量克隆开销。

### 重构

- **`AppState.messages`**: `Vec<ChatMessage>` → `Arc<Vec<ChatMessage>>`，写时复制 (CoW)
  - 读取操作（`.len()` `.iter()` `.last()` `.get()`）通过 `Deref` 零开销透传
  - 变更操作（`.push()` `.pop()` `.clear()` `.drain()`）通过 `Arc::make_mut` 自动 CoW
- **`Session.messages`**: 同步改为 `Arc<Vec<ChatMessage>>`
  - `save_session`: O(n) clone → O(1) `Arc::clone`（引用计数递增）
  - `load_session`: O(1) `Arc::clone` 共享已加载会话数据
- **`serde` 依赖**: 新增 `features = ["rc"]` 以支持 `Arc` 的序列化/反序列化

### 性能收益

| 场景 | 改前 | 改后 |
|------|------|------|
| 会话保存 (100条消息) | O(n) 全量 clone | O(1) refcount |
| 会话切换 | O(n) clone 旧+新 | O(1) refcount ×2 |
| 流式请求 (spawn 路径) | O(n) clone → 修改 | O(1) Arc::clone → CoW 修改 |

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/app.rs` | `AppState.messages` 类型变更 + 全部 mutation/load 站点适配 |
| `src/session.rs` | `Session.messages` 类型变更 + Arc 导入 + `new()` 适配 |
| `src/commands/mod.rs` | 8 处 mutation 适配 + 5 处 clone 适配 + iteration 适配 |
| `src/commands/file_cmd.rs` | 2 处 mutation 适配 + 2 处 iteration 适配 |
| `Cargo.toml` | serde 新增 `rc` feature + 版本 0.5.4 → 0.5.5 |
| `CHANGELOG.md` | v0.5.5 条目 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy + fmt clean)

---

## v0.5.4 (2026-05-22) — 大文件模块化拆分

**目标**: 将两个超大文件拆分为子模块，降低维护成本，提升代码可读性。

### 重构

- **commands.rs (1156行) → commands/mod.rs (874行) + commands/file_cmd.rs (295行)**
  - `file_cmd.rs`: `/read` `/write` `/edit` `/export` 四个文件操作命令 + `resolve_path` + `compute_edit_diff`
  - `mod.rs`: 命令分发 + 流式请求 + 技能执行 + 联网搜索 + 消息管理 + 确认对话框
- **ui/draw.rs (929行) → ui/draw/mod.rs (443行) + ui/draw/chat.rs (425行) + ui/draw/modal.rs (87行)**
  - `chat.rs`: `draw_chat_area` + Markdown 内联解析 + 代码高亮 + 列表/引用块检测
  - `modal.rs`: `draw_confirm_modal` 确认弹窗
  - `mod.rs`: 主布局 `draw()` + 标题栏 + 输入区 + 状态栏 + 语法主题初始化

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/commands/mod.rs` | 新建: 命令模块主文件（分发+流式+技能+消息管理） |
| `src/commands/file_cmd.rs` | 新建: 文件操作命令子模块（read/write/edit/export） |
| `src/commands.rs` | 删除: 已拆分为 commands/ 目录 |
| `src/ui/draw/mod.rs` | 新建: 布局模块主文件（draw+标题栏+输入区+状态栏） |
| `src/ui/draw/chat.rs` | 新建: 聊天区渲染子模块（代码高亮+Markdown） |
| `src/ui/draw/modal.rs` | 新建: 确认弹窗子模块 |
| `src/ui/draw.rs` | 删除: 已拆分为 draw/ 目录 |
| `Cargo.toml` | 版本 0.5.3 → 0.5.4 |
| `CHANGELOG.md` | v0.5.4 条目 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy + fmt clean)

---

## v0.5.3 (2026-05-22) — 代码质量 + 安全加固

**目标**: 消除技术债务，提升代码健壮性。修复死代码、裸 unwrap 调用、逻辑缺陷。

### 代码质量

- **死代码清理**: 移除未使用的 `check_api()` 探测方法（自 v0.3.5 已禁用）和 `api_url()` 辅助方法
- **unwrap 全量替换**: 17 处 `.unwrap()` → `.expect("描述")`，所有潜在 panic 点均有语义化消息
  - 12 处 `RwLock` 访问: `.unwrap()` → `.expect("RwLock poisoned")`
  - 5 处逻辑不变的安全断言: 均加描述性消息
- **unreachable 修复**: `api/mod.rs:359` 的 `unreachable!()` → `anyhow::bail!("max retries exceeded — all attempts failed")`，重试逻辑变更不再隐匿 panic
- **match 穷尽性**: `/help` 中 `_ => continue` → `_ => unreachable!("unknown builtin: {}", c)`，新增命令忘加帮助文案时编译期暴露

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/api/mod.rs` | `unreachable!()` → `bail!()`；移除 `api_url()`/`check_api()` 死代码；RwLock `.unwrap()` → `.expect()` |
| `src/api/types.rs` | `as_mut_str` 保留 `#[allow(dead_code)]`（MCP/工具调用预留 API） |
| `src/commands.rs` | `/help` `_ => continue` → `_ => unreachable!()` |
| `src/app.rs` | 3 处 `.unwrap()` → `.expect()` |
| `src/config.rs` | 1 处 `.unwrap()` → `.expect()` |
| `src/ui/draw.rs` | 2 处 `.unwrap()` → `.expect()` |
| `Cargo.toml` | 版本 0.5.2 → 0.5.3 |
| `CHANGELOG.md` | v0.5.3 条目 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy clean)

---

## v0.5.2 (2026-05-22) — 多模型支持 + 智能模型选择

**目标**: 同一 API Provider 支持多个模型（如 DeepSeek chat/reasoner/V4），用户可列表查看、一键切换、获得按模型精确计费。

### 新功能

- **Provider 多模型 + 中文描述**: 每个模型含 `desc` 字段，`/model` 列出时一目了然
  - **MiMo** (2): `mimo-v2-flash` (轻量快速) + `mimo-v2-pro` (专业推理)
  - **DeepSeek** (4): `deepseek-chat` (V3标准) + `deepseek-reasoner` (R1推理) + `deepseek-v4-flash` (V4轻量) + `deepseek-v4-pro` (V4旗舰)
  - **OpenAI** (3): `gpt-4o-mini` (轻量) + `gpt-4o` (全能) + `gpt-4-turbo` (高性能)
- **`/model`（无参数）列出模型**: 显示当前 Provider 所有模型 + 中文描述 + 各自定价 + 当前选择标注 `(当前)`
- **`/model <name>` 显示描述+定价**: 切换确认消息含一句话描述和价格，未知模型允许但标注"按默认估算"
- **二级补全提示**: 输入 `/model v4` → hint 栏自动显示 `deepseek-v4-flash  deepseek-v4-pro`；`/provider` 同理
- **Provider 切换智能保留**: `/provider deepseek` 时若当前 model 在目标 Provider 列表中存在则保留，不在则回退到默认模型
- **按模型精确计费**: `input_price()`/`output_price()` 三级回退——模型精确匹配 → Provider 默认模型 → 硬编码兜底(2.0/8.0)

### 数据结构

- **新增 `ModelInfo` 结构体**: `name` + `desc` + `input_price_per_mtok` + `output_price_per_mtok`
- **`ProviderPreset` 改造**: 删除 `model`/`input_price_per_mtok`/`output_price_per_mtok` 三字段，改为 `models: &[ModelInfo]` + `default_model_index: usize`
- **新增方法**: `ProviderPreset::default_model()`、`find_model(name)`；`Config::model_info()`

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/config.rs` | ModelInfo(+desc) + ProviderPreset 改造 + PROVIDERS 扩展（3 provider 1 model → 3 provider 2/4/3 models） + 定价三级回退 |
| `src/commands.rs` | /model 重写（列出含desc+切换含desc）+ /provider 智能保留 + update_hint_lines 二级补全(/model + /provider) |
| `src/api/mod.rs` | `update_for_provider` 新增 `model: &str` 参数 |
| `Cargo.toml` | 版本 0.5.1 → 0.5.2 |
| `CHANGELOG.md` | v0.5.2 条目 |
| `PROJECT.md` | 功能清单补充 |
| `OPTIMIZATION.md` | 状态更新 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy clean)

---

## v0.5.1 (2026-05-22) — 代码质量 + Bug 修复

**目标**: 消除代码重复、修复联网搜索缺陷、更新项目文档。

### 代码去重

- **draw.rs 内联解析合并**: `render_markdown_line()` 和 `render_inline_spans()` 共享 45 行内联 Markdown 解析逻辑提取为 `parse_inline_spans()`，消除重复代码

### Bug 修复

- **DDG 搜索结果未注入 AI**: `/search` 命令的 DDG 引擎路径修复——搜索结果现在作为上下文注入对话并流式请求 AI 分析，而非仅显示原始结果
- **search.rs 死代码移除**: `execute_search()` 中 DeepSeek web_search 标记返回路径不可达（调用方已提前处理），移除冗余分支和 `provider` 参数

### 文档更新

- **PROJECT.md**: 依赖列表 `cli-clipboard` → `arboard`，行数/源文件数更新，v0.5.0 新增功能补充
- **CHANGELOG.md**: 新增 v0.5.1 条目

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/ui/draw.rs` | `parse_inline_spans()` 提取，`render_markdown_line`/`render_inline_spans` 共用 |
| `src/search.rs` | 移除不可达 DeepSeek 分支，简化 `execute_search` 签名 |
| `src/commands.rs` | DDG 搜索路径修复：结果注入 → 流式 AI 分析 |
| `PROJECT.md` | 依赖/行数/功能清单更新 |
| `CHANGELOG.md` | 新增 v0.5.1 |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy clean)

---

## v0.4.1 (2026-05-21) — 文档全面更新 + 安全审计 + 路线图扩展

**目标**: README 反映 v0.4.0 全貌，API 泄露扫描与加固，优化路线图从 13 项扩展到 22 项。

### 安全审计

- **API 泄露扫描**: 全项目 `sk-`/`tp-`/`Bearer` 模式匹配，git 历史回溯检查，**0 处真实泄露**
- **OPTIMIZATION.md 脱敏**: N4 撤回示例中的占位密钥替换为 `sk-your-secret-api-key`
- **.gitignore 加固**: 新增 `config.json`、`.env`、`*.key`、`sessions/`、`credentials*` 等 10 条防护规则

### README.md 重写

- 多 Provider 支持表（DeepSeek 默认 / MiMo / OpenAI / 自定义）
- Markdown 全覆盖说明 + 3 套主题热切换
- 余额查询 & 三色预警 + 退出用量汇总
- 4 种 Provider JSON 完整配置示例
- 新增 `/theme`、`/errors`、`/provider` 命令文档
- 缓存策略 ASCII 图解
- 项目结构更新（12 源文件）
- 评分表 + 优化路线图链接
- 对比表新增 10 个对比维度（Markdown/主题/余额/Provider/0clippy 等）

### GitHub 仓库

- **Description**: 更新为多 Provider 定位 + 15 个 Topics（`rust` `terminal` `ai-chat` `deepseek` `openai` `tui` `llm` `coding-assistant` 等）

### OPTIMIZATION.md 扩展

- **优先级全面重排**: P0/P1 全部完成 → 新三层（P1 高优先 / P2 功能增强 / P3 锦上添花）
- **缓存 v3 方案**: W1 日期移出 + W2 System Prompt 拆分 + W3 自适应断点，目标 85-92%
- **10 项用户视角优化**: N1 代理 / N2 编辑重发 / N3 费用预估 / N4 撤回 / N5 输入框扩展 / N6 异步加载 / N7 diff 预览 / N8 通知 / N9 快捷键 / N10 temperature
- **U12 芒果猫 Logo**: ANSI 色块启动 banner 设计方案
- **D1 倒计时**: 21 项 → 约 5 天专注开发

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `README.md` | 完全重写，覆盖 v0.4.0 全部功能 |
| `OPTIMIZATION.md` | +~300 行：缓存 v3 + 10 项用户优化 + 优先级重排 + Logo 设计 + D1 倒计时 |
| `CHANGELOG.md` | 新增 v0.4.1 + v0.5.0 计划 |
| `.gitignore` | 加固 10 条安全规则 |

---

## v0.5.0 (2026-05-22) — 缓存 v3 + 基础设施 + 体验优化

**目标**: 缓存命中率 70% → 85-92%，代理/费用预估/撤回/通知/temperature/异步加载/自适应输入框/diff预览/Logo/clipboard/联网搜索 等 15 项优化一次交付。

### P1 高优先 (4/5 完成)

- **W1 日期移出缓存前缀**: `inject_date_preamble()` 替代 `inject_date_if_needed()`，前置不含 cache_control 的日期消息。预期 +10-15%
- **W2 System Prompt 拆分**: `build_system_stable()`/`build_system_dynamic()` + `build_system_content_split()` 双 block。稳定部分缓存，动态部分不缓存。预期 +5-10%
- **W3 自适应断点布局**: 按对话长度动态分配 4 个 breakpoint（1-3/4-8/9-16/17+），自动跳过 date preamble。预期 +5-8%
- **N1 HTTP/SOCKS5 代理**: `config.json` 新增 `proxy_url` 字段，`reqwest::Proxy::all()` 自动识别 http/https/socks5

### P2 功能增强 (7 项交付)

- **U11 联网搜索**: `/search <关键词>` 命令，DDG 免费 API（无需 Key）+ DeepSeek 原生 `web_search` tool 双引擎。搜索结果注入对话上下文，AI 可基于实时信息回答
- **N3 发送前费用预估**: `util::estimate_tokens()` + `util::format_cost_estimate()`，输入框右侧实时显示 `~¥0.02 ~800tok`，超 20000tok 或 ¥0.5 红色预警
- **N4 Ctrl+Z 撤回**: 移除最后 user+assistant 对话轮次，恢复用户消息到输入框供编辑重发
- **N5 输入框自适应扩展**: 内容超过 3 行时自动扩展（上限半屏），长代码粘贴不再盲打。中文宽度感知（`unicode_width`）
- **N6 syntect 异步加载**: `init_syntax_async()` 在 `std::thread::spawn` 中加载 ~2MB 语法文件，首屏不阻塞。加载完成前代码块纯文本 fallback
- **N7 /edit diff 预览**: `compute_edit_diff()` 生成 unified diff，确认弹窗中红删绿增显示，心里有底再确认写入
- **U12 芒果猫启动 Logo**: ANSI 色块像素画（8×5 猫脸）+ 版本/Provider/Model/余额信息，按任意键进入

### P3 锦上添花 (3 项交付)

- **N8 回复完成通知**: 流式完成时 `\x07` 终端响铃
- **N10 temperature/top_p 可配**: config.json 新增可选 `temperature`/`top_p` 字段，Anthropic/OpenAI 请求透传
- **C3 cli-clipboard → arboard**: Wayland 原生剪贴板支持，替换 `cli-clipboard` 依赖

### 代码清理

- 移除废弃函数 `build_system_content`（被 `build_system_content_split` 替代）
- 移除废弃函数 `inject_date_if_needed`（被 `inject_date_preamble` 替代）
- `maybe_truncate_messages` 简化为纯截断（日期注入统一由 preamble 处理）
- `as_mut_str` 保留并添加 `#[allow(dead_code)]`（公共 API 预留）
- `SyntaxSet`/`Theme` 从 `OnceLock::get_or_init` 同步加载 → `std::thread::spawn` + `OnceLock::set` 异步预加载

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 版本 0.4.0 → 0.5.0 |
| `src/prompt.rs` | W1 `inject_date_preamble()` + W2 `build_system_stable()`/`build_system_dynamic()`/`build_system_content_split()` + W3 自适应断点（跳过 preamble、按长度分 4 档） |
| `src/config.rs` | 新增 `proxy_url`、`temperature`、`top_p`、`web_search` 字段 + `WebSearchConfig` 结构体 + ConfigRaw/save/load/Debug 全部同步 |
| `src/api/mod.rs` | `MiMoClient::new()` 接受 proxy_url + temperature + top_p；`ClientSettings` 扩展；`build_anthropic_request`/`build_openai_request` 透传 temperature/top_p；新增 `send_message_stream_with_search()` (DeepSeek web_search tool) |
| `src/search.rs` | **新增**: DDG HTML 搜索 (`search_ddg`)、DeepSeek web_search 引擎路由、结果格式化 (`format_results`) |
| `src/api/types.rs` | `AnthropicRequest`/`OpenAIRequest` 新增 `temperature`/`top_p` 字段（`skip_serializing_if = "Option::is_none"`） |
| `src/app.rs` | `AppState` 新增 `system_stable`/`system_dynamic`；proxy_url/temperature/top_p 传入 MiMoClient；Ctrl+Z 撤回；终端响铃 `\x07`；`init_syntax_async()` 调用 |
| `src/commands.rs` | `spawn_stream_request` 改用 `inject_date_preamble()` + `build_system_content_split()`；`handle_user_skill` 同；`handle_provider_command` 同步更新 stable/dynamic；`maybe_truncate_messages` 精简 |
| `src/ui/draw.rs` | N3 费用预估渲染（输入框右侧 `~¥ tok` + 阈值红色预警）；N5 自适应输入框（动态 `Constraint::Length`，3~半屏）；N6 语法高亮 fallback；N7 diff 预览（`+ ` / `- ` 颜色标记） |
| `src/ui/mod.rs` | 导出 `init_syntax_async`、`draw_logo` |
| `src/ui/logo.rs` | **新增**: U12 芒果猫 ANSI 色块启动 Logo |
| `src/util.rs` | N3 `estimate_tokens()`（中文 ~1.5/英文 ~0.75 tok/字）+ `format_cost_estimate()` |

### 编译状态

- **0 errors, 0 warnings** (rustc + clippy clean)

---

## v0.4.0 (2026-05-21) — Markdown 渲染完善 + 主题热切换

**目标**: 完成 Markdown 渲染全覆盖 + 3 套内置主题随心切换。

### Markdown 渲染完善 (U3 完成)

- **无序列表**: `- ` / `* ` 开头的行渲染为 `  • ` 前缀 + 缩进
- **有序列表**: `1. ` 开头的行渲染为 `  1. ` 前缀 + 缩进
- **链接**: `[text](url)` 渲染为青色下划线文本，隐藏裸 URL
- **水平分隔线**: `---` / `***` / `___` 渲染为全宽分隔线
- 链接解析同时支持对话区行内和列表项内

### 主题热切换 (U4)

- **主题系统重构**: `ThemeColors` 结构体替代硬编码常量，所有 UI 颜色集中管理
- **3 套内置主题**:
  - `tokyo-night` (默认) — 紫蓝暗色系
  - `nord` — 蓝灰冷色系
  - `catppuccin` — 柔和暖色系
- **`/theme <name>` 命令**: 运行时即时切换主题，无需重启
- **`/theme`** (无参数): 列出所有可用主题，标注当前选择
- 配置持久化: `config.json` 新增 `theme` 字段，重启后保持选择

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 版本 0.3.9 → 0.4.0 |
| `src/ui/theme.rs` | 完全重写: `ThemeColors` 结构体 + `TOKYO_NIGHT`/`NORD`/`CATPPUCCIN` 3 套色板 + `get_theme()`/`theme_names()` |
| `src/ui/mod.rs` | 导出更新: `get_theme`/`theme_names` |
| `src/ui/draw.rs` | 全部函数接受 `&ThemeColors` 参数；新增列表/链接/分隔线渲染；新增 `render_inline_spans()`/`is_hr_line()`/`is_unordered_list()`/`is_ordered_list()` |
| `src/config.rs` | Config/ConfigRaw 新增 `theme` 字段 (默认 `"tokyo-night"`) |
| `src/app.rs` | `/theme` 命令、BUILTIN_COMMANDS 新增 theme、帮助文本、draw 调用传入主题 |

---

## v0.3.9 (2026-05-21) — Markdown 渲染增强

**目标**: 引用块视觉支持 + 粗体/斜体/行内代码渲染。

### 新增功能

- **U8 引用块视觉**: `>` 开头的行渲染为 `│` 竖线 + 缩进 + 斜体蓝灰，与普通对话文字清晰区分
- **U3 内联 Markdown**:
  - `**粗体**` → 加粗样式
  - `*斜体*` → 斜体样式
  - `` `行内代码` `` → 青字 + `#24283b` 深色背景
- 新増 `BLOCKQUOTE_BORDER` (#3d59a1) 和 `INLINE_CODE` (#24283b) 配色常量
- 剔除余额显示 `💰` emoji，保持纯文字风格

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 版本 0.3.8 → 0.3.9 |
| `src/ui/theme.rs` | 新增 `BLOCKQUOTE_BORDER`、`INLINE_CODE` 颜色常量 |
| `src/ui/draw.rs` | 引用块 `│` 渲染、`render_markdown_line()` 函数（粗体/斜体/行内代码）、剔除 💰 |

---

## v0.3.8 (2026-05-21) — 实测体验修复

**目标**: 基于 DeepSeek 实机测试发现的 6 项体验问题，快速响应修复。

### 体验修复 (6/6)

- **F1 首次启动 API 状态引导**: 标题栏 `⊛ ...` → `⊛ 发消息检测API`，新用户一目了然
- **F2 余额货币单位**: DeepSeek 响应中解析 `currency` 字段（CNY→¥, USD→$），余额显示 `💰¥6.33` 而非裸数字
- **F3 余额颜色预警**: 余额 < ¥1 红色、< ¥5 黄色警告、≥ ¥5 绿色，用完前醒目提醒
- **F4 会话名友好化**: `Session::auto_name()` 改为 `{目录名}_{HHMM}` 格式（如 `mimo-opt_1045`），替代无意义的 `session-20594_1045`
- **F5 余额查询失败可见**: 启动或切换 provider 时查询失败，状态栏显示红色错误提示而非静默
- **F6 退出用量汇总**: Ctrl+Q 退出后在终端打印消息数/token/命中率/费用摘要

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 版本 0.3.7 → 0.3.8 |
| `src/app.rs` | F5 余额查询失败 → error_message、F6 退出汇总打印、balance_info 类型改为 (String, f64) |
| `src/api/mod.rs` | F2 解析 currency 字段、返回 (显示文本, 数值) 元组 |
| `src/session.rs` | F4 会话名用目录名 + 时分 |
| `src/ui/draw.rs` | F1 API 状态引导文字、F2 货币符号、F3 余额颜色预警（分三档） |

---

## v0.3.7 (2026-05-21) — 工程化夯实

**目标**: 补齐 P2 工程化短板：结构化日志、API 自动重试、DeepSeek 余额查询、消除隐式错误吞没。

### 结构化日志 (E5 / P2-12)

- 新增依赖 `log = "0.4"` + `env_logger = "0.11"`
- `main.rs`: 初始化 `env_logger`，默认过滤级别 `warn`，可通过 `RUST_LOG=mimo_opt=debug` 启用详细日志
- 17 个日志点位覆盖关键路径：
  - `config.rs`: 加载/保存配置
  - `session.rs`: 加载/保存会话
  - `api/mod.rs`: 请求发送、重试、流式完成（Anthropic/OpenAI）
  - `app.rs`: 缓存断点计数、技能执行、会话切换、消息完成
  - `file_ops.rs`: 文件写入/编辑/备份
- `util.rs`: 系统时钟异常时输出 `log::warn!`（替换静默 `unwrap_or_default`）

### API 自动重试 (E1 / P2-9)

- `send_message_stream()` 新增循环重试逻辑（最多 3 次）
- 重试条件：5xx 状态码 / 429 rate limit / 网络错误（timeout、connection reset）
- 指数退避间隔：2s → 4s → 8s
- 非幂等错误（4xx）不重试，直接返回错误
- 重试和错误日志清晰可追踪

### DeepSeek 余额查询 (E4)

- `MiMoClient` 新增 `query_deepseek_balance()` 方法
- 启动时 provider 为 deepseek 时自动查询余额（`GET /user/balance`）
- 支持两种余额字段解析：`balance_infos[0].total_balance` 和 `balance` 数值
- `/provider deepseek` 切换时自动查询余额
- 标题栏显示 `💰 {balance}` 余额信息

### Bug 修复

- **P3-12** 修复 Anthropic 流式解析中 `output_tokens += 1` 的粗略估算，改为完全依赖 `message_delta` 的准确 usage 计数
- **P2-10** `now_secs()` 中 `unwrap_or_default()` 改为 `unwrap_or_else(|e| log::warn!(...))`，系统时钟异常不再静默吞错

### 代码质量

- 移除 `ui/draw.rs` 中 2 个未使用的辅助函数
- 编译：0 errors, 0 warnings (clippy clean)

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 新增 log + env_logger 依赖，版本 0.3.6 → 0.3.7 |
| `src/main.rs` | 初始化 env_logger |
| `src/util.rs` | `now_secs()` 系统时钟异常日志 |
| `src/api/mod.rs` | 自动重试循环、DeepSeek 余额查询、流式完成日志、移除 output_tokens 粗略估算 |
| `src/config.rs` | 加载/保存日志 |
| `src/session.rs` | 加载/保存日志 |
| `src/file_ops.rs` | 写入/编辑/备份日志 |
| `src/app.rs` | balance_info/balance_rx 字段、缓存断点/技能执行/会话切换日志、消息完成日志 |
| `src/ui/draw.rs` | 标题栏余额展示、移除 2 个未使用函数 |

---

## v0.3.6 (2026-05-20) — 代码去重 + 架构优化

**目标**: 消除重复代码，提取公共模式，提升可维护性。零功能变更，零行为变更。

### 重构

- **R1 SSE 流式解析提取**: `stream_anthropic()` 和 `stream_openai()` 共享的 UTF-8 安全解码 + SSE 事件分割逻辑提取为 `process_sse_stream()` 泛型函数 + `SseAction` enum。两套流解析器各减少 ~40 行重复代码
- **R2 代码块提取统一**: `collect_code_blocks()` 从内联解析改为调用 `extract_code_blocks_from_text()`，消除第 4 套代码块解析实现。`extract_last_code_block()` 同步简化
- **R3 日期注入 + 异步调度提取**: 3 处重复的"首条消息注入日期 + clone 消息 + 缓存断点 + spawn"模式提取为 `inject_date_if_needed()` + `spawn_stream_request()` 两个函数。正常发送 / 确认发送 / `send_to_mimo` 三路径统一
- **R4 内置命令常量**: `BUILTIN_COMMANDS` 常量替代 `update_hint_lines()` 中的硬编码列表，消除与 `/help` 文本的不一致风险

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/api/mod.rs` | R1: `process_sse_stream()` + `SseAction` enum，两套流解析器精简 |
| `src/app.rs` | R2/R3/R4: 代码块统一、日期注入提取、命令常量、`send_to_mimo` 精简为 1 行 |
| `Cargo.toml` | 版本号 0.3.5 → 0.3.6 |

### 回滚指南

如需回滚到 v0.3.5：
```bash
git log --oneline  # 找到 v0.3.5 checkpoint commit
git revert HEAD    # 或 git reset --hard <checkpoint_hash>
```

---

## v0.3.5 (2026-05-19) — UX 优化六连

### 新功能

- **U3 长消息发送确认**: 输入超过 5 行的普通消息时弹出确认弹窗，防止误触浪费配额。取消发送时消息自动恢复到输入框
- **U4 对话轮次分隔线**: 不同问答轮次之间用虚线 `─ ─ ─` 分隔，长对话中快速定位轮次边界
- **U6 会话导出 Markdown**: `/export [path]` 命令导出当前会话为 `.md` 文件，含模型/token/费用元信息
- **U9 错误历史**: API 错误自动记录到 `error_history`（最多 20 条），`/errors` 命令查看
- **U10 终端最小尺寸检查**: 窗口 < 60×20 时显示居中红色警告，避免布局崩坏
- **U14 启动省配额**: 移除启动时 `check_api()` 探测请求，首条消息自然检测 API 可用性

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/app.rs` | `ConfirmAction::SendMessage`、长消息检查、`/export` + `/errors` 命令、错误历史、移除 `check_api` |
| `src/ui/draw.rs` | 轮次分隔线渲染、终端尺寸检查 |
| `src/api/mod.rs` | `check_api` 加 `#[allow(dead_code)]` |

---

## v0.3.4 (2026-05-19) — Bug 修复 + UX 优化 + 零警告

**目标**: 修复 v0.3.3 审计发现的 bug，实现高优先级 UX 优化，clippy 零警告。

### Bug 修复 (8/9)

- **B3 `/read` 行范围解析错误提示改善**: 错误信息改为"无效起始行号/结束行号"，更明确
- **B6 会话 token 持久化**: `Session` 新增 `total_input_tokens`/`total_output_tokens`/`total_cache_creation_tokens`/`total_cache_read_tokens`/`total_cost` 5 个字段（`#[serde(default)]` 向后兼容），`save_session()` 写入，启动和 F2 切换时恢复
- **B7 搜索高亮对比度提升**: 背景色从 `#3b4261` 改为 `#565f89`（亮一档），搜索匹配项清晰可辨
- **B8 日期注入修复**: 两处 `messages.len() == 1` 改为检查 `first.role == "user"` 且不包含 `[Current date:`，避免在 assistant 消息上重复注入
- **B1/B2/B4/B5**: 在 v0.3.3 审计前已修复（SSE 双分隔符、stream_buffer 扫描、消息数检查、死参数移除）

### 新功能

- **U1 Ctrl+V 粘贴**: 从剪贴板读取内容插入到光标位置，`cli_clipboard::get_contents()` 已在依赖中
- **U2 代码块深色背景**: `theme.rs` 新增 `CODE_BG = #1a1b26`，所有代码块行、边框（`│`/`┌`/`└`）和流式 buffer 中的代码块均应用深蓝黑背景

### 代码质量

- **27 个 clippy 警告 → 0 个**: 自动修复 19 个（`collapsible_match`×9 + `needless_borrow`×6 + `manual_is_multiple_of`×3 + `useless_format`×1），手动修复 8 个（`manual_strip`×5 + `let_underscore_future`×1 + `explicit_counter_loop`×1 + `unnecessary_sort_by`×1）
- 编译：0 errors, 0 warnings

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `src/app.rs` | Ctrl+V 粘贴、B3/B6/B7/B8 修复、clippy 修复 |
| `src/session.rs` | 5 个 token 字段、`sort_by_key` |
| `src/ui/draw.rs` | 代码块深色背景、搜索高亮 |
| `src/ui/theme.rs` | 新增 `CODE_BG` 常量 |
| `src/file_ops.rs` | `strip_prefix` 修复 |

---

## v0.3.3 (2026-05-19) — 实机测试 + 用户优化审查

**审查范围**: 全量代码审查 + clippy 检查 + 用户视角评估

### Bug 发现

- **B1 (P1) SSE 分隔符兼容性**: `api/mod.rs` 中两套 SSE 解析器硬编码 `\n\n` 事件分隔符，不兼容使用 `\r\n\r\n` 的服务器（部分代理/负载均衡器），导致事件无法分割、token 静默丢失
- **B2 (P2) Ctrl+Y 对生成中代码块无效**: `collect_code_blocks()` 仅扫描 `state.messages`，忽略 `stream_buffer`。而 `extract_last_code_block()` 会检查两者，行为不一致
- **B3 (P2) `/read` 行范围解析静默吞错**: `unwrap_or(0)` 在 parse 失败时 fallback 为读取整个文件，而非报错
- **B4 (P2) `send_to_mimo` 缺少消息数检查**: 正常路径有 `maybe_truncate_messages()`，但 `send_to_mimo` 内部 clone + 追加后直接发送，无上限保护
- **B5 (P2) `is_git_repo` 死参数**: 在 `scan_dir_recursive` 递归链路中传递但从未使用
- **B6 (P3) 会话 token 计数不持久**: 切换/重启会话后 `total_input_tokens` 等归零，`Session` 结构体不含累计字段
- **B7 (P3) 搜索高亮对比度不足**: 背景色 `#3b4261` 与终端背景 `#1a1b26` 差异极小，匹配项几乎不可见
- **B8 (P3) `send_to_mimo` 日期注入角色错误**: `len()==1` 时可能在 assistant 消息上追加日期（全新会话先 `/read` 的场景）
- **B9 (P3) 26 个 clippy 警告积压**: `collapsible_match`×9 + `needless_borrow`×6 + `manual_strip`×4 + `manual_is_multiple_of`×3 + 其他×4

### 用户优化建议 (15 条)

**高优先级 (★★★★★)**:
- **U1 Ctrl+V 粘贴**: `cli-clipboard` 已在依赖中，仅需接 crossterm 键盘事件
- **U2 代码块深色背景**: 加 `bg = #1a1b26` 即可大幅提升视觉区分，UI_DESIGN 已规划

**中高优先级 (★★★★)**:
- **U3 长消息发送确认**: 可配置开关，防止误触浪费配额
- **U4 对话轮次分隔线**: 虚线分隔不同问答轮次，UI_DESIGN 已规划
- **U5 API 余额查询**: 启动时查余额显示在标题栏
- **U6 会话导出 Markdown**: `/export [path]` 命令

**中优先级 (★★★)**:
- **U7 输入框多行自动扩展**: 粘贴长代码时动态增高到半屏
- **U8 Markdown 粗体/斜体/列表渲染**: `**text**` → 粗体，`- item` → 列表前缀
- **U9 错误信息历史**: `/errors` 查看最近错误，不再一闪而过
- **U10 终端尺寸检查**: 窗口 < 60×20 时显示居中警告

**低优先级 (★★/★)**:
- **U11** 会话侧边栏、**U12** 流式 token 精度、**U13** temperature/top_p 可配置、**U14** check_api 省配额、**U15** 引用块视觉区分

### 综合评分: 7.7 → 8.1 (完成 P1 bug 修复 + 前 4 项 UX 优化后)

---

## v0.3.2 (2026-05-19)

**里程碑**：P0/P1 安全与体验全部到位，项目评分 6.2 → 7.7。

### P0 安全修复

- **命令注入防护**：新增 `shell_escape()` 函数，对技能 `{args}` 占位符替换做平台适配转义
  - Unix/sh：单引号包裹 + 内部单引号 `'\''` 转义
  - Windows/cmd：`^` 转义 `%^&<>|"()!` 等特殊字符
  - 同时应用于 `{args}` 和无占位符时的 args 追加

### P1 核心体验

- **`/model <name>` 命令**：运行时切换模型（如 `deepseek-chat` → `deepseek-reasoner`），不中断会话，自动保存配置
- **`/provider <name>` 命令**：运行时切换 API 提供商预设，自动更新 endpoint / auth / 格式 / 费率，重建 system prompt 的 AI 身份
- **长对话自动管理**：消息数超过 200 条自动截断保留最近 150 条，160 条时状态栏警告，状态栏新增消息计数 `✉ N`
- **spawn 任务取消机制**：`tokio::select!` + `oneshot` channel，Ctrl+C / Esc 真正中止 HTTP 流式请求，不再后台空跑消耗配额

### 架构改进

- `MiMoClient` 内部可变字段统一收敛到 `RwLock<ClientSettings>`，支持运行时 model / provider 热切换
- `send_message_stream` / `check_api` 使用 block scope 确保 `RwLockReadGuard` 在 `.await` 前释放
- 三个 spawn 点（普通消息 / 技能执行 / send_to_mimo）全部接入取消机制
- `AppState` 新增 `cancel_tx` 字段，`Done` / `Error` 处理中自动清理

---

## v0.3.1 (2026-05-19)

**修复**：DeepSeek API 实测发现的 4 个 bug。

### Bug 修复

- `Usage` 结构体新增 `#[serde(alias)]`：DeepSeek 返回 `prompt_tokens`/`completion_tokens`，原字段名不匹配导致 token 计数永远为 0
- `build_openai_request` 补充 system 参数：OpenAI/DeepSeek 路径下系统提示词被丢弃，模型收不到上下文
- SSE 解析器新增 `partial` 缓冲：跨 chunk 分割的 SSE 事件片段不再丢失
- `calculate_cost` 改为按 provider 费率动态计算，不再写死 MiMo 价格

### 代码质量

- `% 5 == 0` → `is_multiple_of(5)`（clippy）
- SSE 解析器两处循环体内 `remaining` 改为 `String` 持有（避免 &str 生命周期陷阱）

---

## v0.3.0 (2026-05-19)

**里程碑**：多 Provider 支持，DeepSeek API 正式集成。

### Provider 预设系统

- `Config` 新增 `provider` 字段（`"mimo"` / `"deepseek"` / `"openai"` / `"custom"`）
- 新增 `ProviderPreset` 结构体 + `PROVIDERS` 常量表，3 个内置预设
- 每个预设自带：base_url / model / auth_type / api_format / 费率
- `input_price()` / `output_price()` 按 provider 返回对应费率
- 旧配置文件无 `provider` 字段自动识别为 `"custom"`，向后兼容

### DeepSeek API 正式集成

- DeepSeek 预设：`https://api.deepseek.com` / `deepseek-chat` / bearer / openai
- OpenAI 请求新增 `stream_options.include_usage`，DeepSeek 流式返回真实 token 用量
- OpenAI SSE 解析器：从 stream 事件中捕获 `usage`，不再永远返回 0
- 费率：输入 ¥1/百万 token，输出 ¥2/百万 token

### 系统提示词 & UI 适配

- `build_system_prompt_text()` 根据 provider 输出不同 AI 身份（DeepSeek / ChatGPT / MiMo）
- 标题栏动态显示 `{Provider}-OPT`（如 `DeepSeek-OPT`）
- 计费公式改为调用 `config.input_price()` / `config.output_price()`

### 启动引导优化

- 首次运行提示扩展为 5 种方案：MiMo Token Plan / DeepSeek / OpenAI / MiMo API / 自定义
- 每种方案含完整 JSON 配置示例和密钥获取链接

### 支持的 API

| API | auth_type | api_format | 状态 |
|-----|-----------|------------|------|
| MiMo Token Plan | anthropic | anthropic | ✅ 默认 |
| DeepSeek | bearer | openai | ✅ 已集成 |
| OpenAI | bearer | openai | ✅ 已集成 |
| MiMo API | bearer | openai | ✅ 可用 |
| GLM (智谱) | bearer | openai | ⏳ provider: "custom" |
| 通义千问 | bearer | openai | ⏳ provider: "custom" |

---

## v0.2.0 (2026-05-19)

**里程碑**：首个具备多 API 格式扩展能力的稳定版本。

### 代码去重

- 新增 `src/util.rs` 共享模块：`now_secs()`（时间戳）、`get_cwd()`（工作目录）
- 5 处内联时间戳 → `now_secs()`（session.rs / file_ops.rs / app.rs）
- 5 处会话保存代码块 → `save_session()` 函数统一调用
- 5 处 `current_dir()` + 错误处理 → `get_cwd()` 统一调用
- 2 处 "已取消" 消息创建 → `push_cancelled_message()` 统一调用
- `SystemContent` 补充缺失的 `Clone` derive

### API 多格式架构

- `Config` 新增 `api_format`（`"anthropic"` | `"openai"`，默认 `"anthropic"`）
- `Config` 新增 `max_tokens`（默认 `4096`，可按需调整）
- `MiMoClient` 按 `api_format` 自动选择端点和请求/响应格式
- Anthropic 格式：`/v1/messages` + 独立流式解析器（不变）
- OpenAI 格式：`/v1/chat/completions` + 独立流式解析器
- 两套解析器完全独立，不共用逻辑，互不干扰
- `auth_type` 和 `api_format` 正交组合：4 种搭配自由选择
- 首次运行提示新增"方案三"：第三方 OpenAI 兼容 API 配置

### 支持的 API

| API | auth_type | api_format | 状态 |
|-----|-----------|------------|------|
| MiMo Token Plan | anthropic | anthropic | ✅ 默认 |
| MiMo API | bearer | openai | ✅ 可用 |
| DeepSeek | bearer | openai | ⏳ 待实测 |
| GLM (智谱) | bearer | openai | ⏳ 待实测 |
| 通义千问 | bearer | openai | ⏳ 待实测 |

### 回滚说明

- API 格式：`types.rs` 中 `AnthropicRequest` 改回 `ChatRequest`，删除 OpenAI 类型，`mod.rs` 恢复单格式
- 去重改动：`save_session` / `push_cancelled_message` / `get_cwd` 调用处还原为内联代码，删除 `util.rs`
- Config 新字段为 `Option`，旧配置文件无需迁移

---

## v0.1.0 (2026-05-18)

### 项目初始化 + 核心功能

- Rust 项目骨架（ratatui + crossterm + tokio + reqwest）
- MiMo API 流式对话（SSE 逐 token，UTF-8 跨 chunk 安全解码）
- 终端界面：标题栏 / 对话区 / 输入框 / 状态栏（Tokyo Night 配色）
- 配置文件自动创建 + API 状态探测
- Ctrl+Q 退出 / Ctrl+C 中断生成

### 审查修复（两轮，共 29 项）

**高优先级**：流式死循环修复 / API 错误传递到 UI / scopeguard 终端清理 / UTF-8 安全解码 / check_api 不浪费配额

**中优先级**：Unicode 宽度计算 / Arc 共享客户端 / token 精确计数 / spinner 动画 / Ctrl+C 改为中断不退出

**低优先级**：光标编辑 / 首次运行提示 / 状态栏 IO 分开计数

### Phase 2：缓存 + 技能 + 双认证

- Anthropic prompt caching：system prompt `cache_control` + Usage 缓存字段解析 + 对话前缀断点
- 缓存命中率 `♻ XX%` 状态栏实时显示
- 技能系统：`/命令名` 执行 shell 命令，输出自动发给 MiMo 分析
- 双 API 认证：`auth_type` 支持 `anthropic` / `bearer`

### Phase 4：P0 优化

- 缓存命中率 v2：渐进式多断点 + system prompt 去日期 + 命中率公式修正（15-30% → 70-85%）
- 代码块语法高亮：syntect + 自定义 Tokyo Night 主题（22 种 scope）
- 输入历史浏览：↑/↓ 浏览已发送消息

### Phase 5：P1 核心体验

- 技能增强：`{args}` 参数传递 / 内置 5 个默认技能 / 输出截断 + 计时 / 输入提示
- 聊天区滚动：PageUp/PageDown + 滚动指示器
- 消息搜索：Ctrl+F 实时搜索 + 高亮 + 跳转
- 启动优化：OnceLock 缓存 syntect theme

### Phase 6：文件操作

- `/read` 读取文件（带行号，支持行范围），自动发给 MiMo 分析
- `/write` 提取最后代码块写入文件（自动备份）
- `/edit` 精确字符串替换（自动备份）
- 路径沙箱：禁止穿越、禁止绝对路径、.git 不可写

### Phase 7：确认机制

- 破坏性操作（写入/编辑/清空/退出）弹出居中确认弹窗
- y 确认 / n 取消 / d 展开详情

### Phase 8：Content enum

- `ChatMessage.content` 从 `String` 改为 `enum Content { Text(String) }`
- `#[serde(untagged)]` 保证 API 向后兼容
- 为 Image / ToolUse / ToolResult 变体预留

### Phase 9：附加功能

- Ctrl+Y 复制最后一个代码块到剪贴板
- `/skills` / `/addskill` / `/rmskill` TUI 内管理技能

### Phase 10：多会话持久化

- Session 结构 + JSON 存储（`~/.config/mimo-opt/sessions/`）
- Ctrl+N 新建 / F2 切换 / 每 5 条自动保存 / 退出自动保存
- 标题栏显示会话名

### Phase 3：双 API 认证

- `auth_type` 支持 `anthropic`（api-key header）/ `bearer`（Authorization: Bearer）
- 首次运行提示展示两种配置方案

### 新增依赖

scopeguard / unicode-width / syntect / cli-clipboard

---

## 下一步计划

### 已完成 25/25 项（详见 [OPTIMIZATION.md](OPTIMIZATION.md)）

- v0.5.0: 缓存 v3 + 代理 + 搜索 + 费用预估 + 撤回 + 自适应输入 + 异步高亮 + diff + 通知 + temperature + Logo + arboard + 精确 token
- v0.5.2: 多模型支持
- v0.5.3: 代码质量 5 项（死代码/unwrap/unreachable/match 穷尽性/fmt）
- v0.5.4: 大文件模块化拆分（commands + draw）
- v0.5.5: Arc<Vec<ChatMessage>> 零拷贝共享
- v0.5.6: Shell 管道集成
- v0.5.7: 单元测试 80 个（8 模块覆盖）

### v0.6.0 候选

**P2 体验（2 项）**:
- [ ] U13: 长对话上下文压缩（接近上限时自动摘要旧消息，保留关键细节）
- [ ] U1: 会话侧边栏（Ctrl+B，UI_DESIGN 已规划）

**P3 补充（2 项）**:
- [ ] N9: 快捷键可配置（keybindings.json）
- [ ] D1: 桌面端 Tauri 迁移（core/gui 分层，feature flag）

**待定**:
- [ ] GLM / 通义千问 / Kimi 等平台预设完善
- [ ] MCP 支持
