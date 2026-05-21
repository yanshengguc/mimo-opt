# MiMo-OPT 更新日志

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

## v0.5.0 (计划中) — 缓存 v3 + 用户视角体验补全

**目标**: 缓存命中率 70% → 90%，代理/编辑重发/费用预估/撤回 等 10 项用户呼声最高的体验优化。

### P1 高优先 (5 项)

- **N1 HTTP/SOCKS5 代理**: config.json 新增 `proxy.url` 字段，reqwest 原生支持，国内网络兜底
- **W1 日期移出缓存前缀**: `inject_date_preamble()` 替代 `inject_date_if_needed()`，messages[0] 跨日不失效。预期 +10-15%
- **W2 System Prompt 拆分**: `build_system_stable()`/`build_system_dynamic()` 分离，多项目切换缓存不丢。预期 +5-10%
- **W3 自适应断点布局**: 按对话长度动态分配 4 个 breakpoint。预期 +5-8%
- **E2 单元测试**: file_ops/config/prompt/cost/session 测试补齐

### P2 功能增强 (10 项)

- **N2 编辑重发**: ↑ 调出上条消息到输入框，编辑后重发
- **N3 发送前费用预估**: 输入框右侧实时显示 ~¥/tok 数，超过阈值黄色警告
- **N4 Ctrl+Z 撤回**: 移除最后一条 user+assistant 对话轮次
- **N5 输入框自适应扩展**: 内容超出时自动扩展，上限半屏
- **N6 syntect 异步加载**: 启动不阻塞，高亮延后加载
- **N7 /edit diff 预览**: 确认弹窗红删绿增 unified diff
- **U11 联网搜索**: `/search` 命令，DeepSeek 原生 > DDG fallback
- **U1 会话侧边栏**: Ctrl+B 呼出
- **U12 芒果猫 Logo**: ANSI 色块启动 banner
- **C1/C2**: clone 优化 + 文件拆分

### P3 锦上添花 (6 项)

- **N8 回复完成通知**: 终端响铃 + 桌面通知
- **N9 快捷键可配置**: keybindings.json
- **N10 temperature/top_p 可配**: config 透传
- **U9/C3/C4**: Shell 管道 + clipboard + token 精度

### 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 版本 0.4.0 → 0.5.0，新增 notify-rust 依赖 |
| `src/prompt.rs` | W1 `inject_date_preamble()` + W2 拆分 + W3 自适应 |
| `src/config.rs` | `proxy`、`web_search`、`temperature`、`top_p` 字段 |
| `src/api/mod.rs` | proxy 集成 + temperature/top_p 透传 |
| `src/commands.rs` | `/search` + `/edit` diff 确认 + N2 编辑重发 + N4 撤回 |
| `src/app.rs` | input_history、undo_stack、异步 syntect、通知触发 |
| `src/ui/logo.rs` | **新增**: 芒果猫色块 Logo |
| `src/ui/draw.rs` | 输入框动态扩展 + 费用预估 + diff 弹窗 + Logo 展示 |
| new `src/search.rs` | DDG API + 结果抓取 |
| new `tests/` | file_ops / config / prompt 单元测试 |

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

### v0.4.1+ 待完成

**P2 工程化（剩余）**:
- [ ] 单元测试覆盖（优先 file_ops / config / prompt / cost / session）
- [ ] app.rs 模块拆分（~1900 行 → app / prompt / scanner / cost / commands）
- [ ] GLM / 通义千问 / Kimi 等平台预设完善

**P3 锦上添花**:
- [ ] U1: 会话侧边栏（Ctrl+B，UI_DESIGN 已规划）
- [x] U3: Markdown 渲染增强（粗体/斜体 v0.3.9 + 列表/链接/分隔线 v0.4.0）
- [x] U4: 主题热切换（v0.4.0: Tokyo Night / Nord / Catppuccin + /theme 命令）
- [x] U8: 引用块视觉支持（v0.3.9: `>` 竖线+缩进+斜体）
- [ ] U9: Shell 管道集成（`echo "..." | mimo-opt --prompt`）

**已决定后续再做**:
- [ ] **P2-8** 桌面端迁移（Tauri）— 终端版先交付
- [ ] MCP 支持
