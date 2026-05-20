# MiMo-OPT 更新日志

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

### v0.3.3 Bug 修复（优先）
- [ ] B1: SSE `\n\n` → 兼容 `\r\n\r\n` 分隔符（P1，影响一类服务器兼容性）
- [ ] B2: `collect_code_blocks` 纳入 stream_buffer（P2，Ctrl+Y 行为一致性）
- [ ] B3: `/read` 行范围 parse 失败报错而非静默 fallback（P2）
- [ ] B4: `send_to_mimo` 加消息数检查（P2）
- [ ] B5: 移除 `is_git_repo` 死参数（P2）
- [ ] B6: Session 持久化 token 计数（P3，但用户体感重要）
- [ ] B9: 清理 26 个 clippy 警告

### v0.3.3 用户急迫优化（低成本高收益）
- [ ] U1: Ctrl+V 粘贴支持（~15 行，`cli-clipboard` 已在依赖中）
- [ ] U2: 代码块深色背景 `bg = #1a1b26`（~5 行，UI_DESIGN 已规划）
- [ ] U4: 对话轮次分隔线（~10 行，UI_DESIGN 已规划）

### v0.4.0 功能增强
- [ ] U5: API 余额查询（启动时 `GET /user/balance` 显示在标题栏）
- [ ] U6: 会话导出 Markdown（`/export [path]` 命令）
- [ ] U8: Markdown 粗体/斜体/列表/行内代码渲染
- [ ] U3: 发送前确认（可配置开关）

### 工程化
- [ ] 单元测试覆盖（优先 file_ops / config / prompt / cost / session）
- [ ] API 请求自动重试（网络瞬时故障，最多 3 次指数退避）
- [ ] 结构化日志（`log` + `env_logger`，关键路径 info/debug/warn）
- [ ] app.rs 模块拆分（~1600 行 → app / prompt / scanner / cost / commands）
- [ ] GLM / 通义千问 / Kimi 等平台预设完善
- [ ] **P2-8** 桌面端迁移（Tauri）— 终端版先交付
