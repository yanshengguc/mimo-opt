# MiMo-OPT 更新日志

## 2026-05-18

### 项目初始化 + 首个可运行版本

**搭建项目骨架**
- 初始化 Rust 项目（Cargo.toml + 目录结构）
- 技术栈确定：Rust + ratatui + crossterm + tokio + reqwest
- MiMo Token Plan API 对接（Anthropic Messages 兼容格式）

**实现核心功能**
- MiMo API 流式对话（SSE 逐 token 输出）
- ratatui 终端界面：标题栏 / 对话区 / 输入框 / 状态栏
- Tokyo Night 配色方案（15 个颜色常量）
- 配置文件自动创建（`~/.config/mimo-opt/config.json`）
- API 状态探测（标题栏 ⊛ API OK / 错误信息）
- Ctrl+Q 退出 / Ctrl+C 中断生成 / Esc 中断生成

**UI 设计文档**
- 编写 `UI_DESIGN.md` 界面视觉设计稿
- 编写 `PROJECT.md` 项目需求文档
- 编写 `HANDOFF.md` AI 交接文档

### 第一轮审查修复（9 个编译错误）

- 修复 `tx` 变量作用域问题（未传入 run_loop）
- 修复 `MiMoClient` 私有字段（改为 `pub`）
- 修复 `balance_text` / `model_text` 移动后借用（先计算长度再创建 Span）
- 移除未使用的 `List` / `ListItem` 导入

### 第二轮全面审查修复（20 个问题）

**高优先级（5 项）**
- 流式完成检测死循环风险 → 删除 `is_empty + continue` 逻辑，统一用 `StreamResult` 枚举
- API 错误静默丢弃 → 新增 `StreamResult::Error`，错误传递到 UI 显示
- 终端清理保护 → 加 `scopeguard`，异常退出也能恢复终端
- `check_balance` 浪费配额 → 改为 `check_api`（只探测，不查余额）；token 用量从 API 响应事件获取
- UTF-8 跨 chunk 分片 → 改用 `Vec<u8>` 原始缓冲区 + `std::str::from_utf8` 安全解码

**中优先级（6 项）**
- Unicode 宽度计算错误 → 引入 `unicode-width`，布局计算改用 `UnicodeWidthStr::width()`
- 多次 clone MiMoClient → 改用 `Arc<MiMoClient>` 共享
- token 估算极不准确 → 使用 API 响应的 `input_tokens` / `output_tokens`，费率改为 MiMo 官方定价
- 流式空白期无反馈 → generating 但 buffer 为空时显示 `⠋ thinking...`
- spinner 最后一帧跳变 → `☑` 改为 `⠏`，保持 braille 旋转一致
- Ctrl+C 直接退出无确认 → `Ctrl+Q` 退出，`Ctrl+C` 只中断生成

**低优先级（5 项）**
- 删除 draw_chat_area 中重复的 if/else 分支（死代码）
- 输入框支持 Home/End/左右/Delete/Ctrl+A/E 光标编辑
- 首次运行提示 API Key 格式和获取链接
- 状态栏显示 input/output token 分开计数（↓↑）
- draw 函数签名改为 `&AppState`（只读不需 &mut）

**新增依赖**
- `scopeguard` — 终端清理保护
- `unicode-width` — Unicode 字符宽度计算

### Phase 2：缓存优化 + 技能系统 + 双 API 认证

**缓存命中率优化（P0 三项）**
- `ChatRequest` 新增 `system: Option<Vec<SystemContent>>` 字段 + `CacheControl` / `SystemContent` 结构体
- 固定 system prompt：MiMo 身份 + 输出格式 + 项目文件扫描（100 个文件，排除 .git/node_modules 等）+ 日期
- `Usage` 新增 `cache_creation_input_tokens` / `cache_read_input_tokens`，SSE 事件解析提取
- 对话前缀缓存断点：messages ≥ 4 时标记 message[3] 为 `cache_control: ephemeral`
- 状态栏显示 `♻ XX%` 缓存命中率（绿色）
- 日期变化时自动重建 system prompt，同日内复用缓存

**请求层优化**
- URL 预计算：`messages_url` 字段在 `new()` 时构建，不再每帧 `format!`
- reqwest 连接池：`timeout=120s`、`pool_idle_timeout=90s`、`tcp_keepalive=60s`
- `build_request()` 方法抽取，消除 `check_api` 和 `send_message_stream` 重复代码
- `StreamResult::Done` 新增 `cache_creation_tokens` / `cache_read_tokens`（改 `u64`）

**技能系统（新增）**
- `Config` 新增 `skills: HashMap<String, String>`，配置文件定义 shell 命令映射
- Ctrl+Enter 输入 `/命令名` 时执行对应 shell 命令（Windows: `cmd /C`，其他: `sh -c`）
- 执行输出显示在对话区，然后自动发送给 MiMo 分析
- 未知技能显示错误提示

**双 API 认证（新增）**
- `Config` 新增 `auth_type: String`（`anthropic` / `bearer`），默认 `anthropic`
- `MiMoClient` 新增 `set_auth_headers()`：anthropic 用 `api-key` + `anthropic-version`，bearer 用 `Authorization: Bearer`
- 首次运行提示展示两种 API 配置方案

**工程质量**
- 光标移动改用 `char.len_utf8()`，多字节字符不再 -1 panic
- `#[allow(dead_code)]` 抑制 theme.rs 6 个颜色常量 + mod.rs `base_url`/`api_url` 警告
- 编译通过，0 warnings

**文档**
- 创建 `README.md` — 项目介绍、功能对比、使用说明（含测试阶段提示 + QQ: 2391859666）
- 创建 `OPTIMIZATION.md` — 完整优化路线图（12 项已完成 + 待做方向）
- Git 初始化 → 推送到 https://github.com/yanshengguc/mimo-opt

---

### 缓存策略评审（2026-05-19）

**现状诊断**：
- 静态断点 `messages[3]` 导致长对话命中率仅 15-30%，远低于宣称的 70-90%
- System prompt 含日期，缓存每天失效一次
- 命中率公式用 `cache_read/(creation+read)` 持续走低，不反映真实节省

**已写入 OPTIMIZATION.md 的 P0-0 方案**（三项子任务，一次完成）：
1. 渐进式多断点：`messages[0]` + 每 6 条追加一个断点
2. System prompt 去日期：日期移入首条 user message
3. 命中率公式修正：`cache_read / (input_tokens + cache_read)`

预期提升：命中率 15-30% → 70-85%，不改模型看到的内容。

---

## 2026-05-19

### Phase 4：P0 优化（全部完成）

#### P0-0 缓存命中率 v2

**改动文件**：[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)

**子任务 1 — 渐进式多断点**
- 改动：`apply_cache_breakpoints()` 函数（app.rs 约 612-625 行）
- 之前：只在 `messages[3]` 设一个静态断点
- 之后：`messages[0]` 始终设断点，之后每 6 条消息追加一个断点
- 验证：对话 5 轮后，状态栏 `♻` 应稳定在 70% 以上

**子任务 2 — System prompt 去日期化**
- 改动位置 1：`build_system_prompt_text()` 签名从 `(project_tree, date)` 改为 `(project_tree)`，末尾不再拼入日期
- 改动位置 2：`AppState` 结构体删除 `system_prompt_cached_date` 字段
- 改动位置 3：`run()` 初始化处删除 `system_prompt_cached_date` 赋值
- 改动位置 4：普通消息发送路径（~327 行），删除日期变化检测逻辑，改为 `state.messages.len() == 1` 时在首条消息追加 `[Current date: YYYY/MM/DD]`
- 验证：system prompt 缓存跨天存活，不再因日期变化失效

**子任务 3 — 命中率公式修正**
- 改动：`draw_status_bar()` 中缓存命中率计算（draw.rs ~437-443 行）
- 之前：`cache_read / (cache_creation + cache_read)`，分母持续增长导致命中率虚低
- 之后：`cache_read / (input_tokens + cache_read)`，反映缓存帮省了多少比例的输入成本
- 验证：首次请求 ♻ 0%，第二条起跳到 30-60%，5 轮后稳定 70%+

#### P0-1 代码块语法高亮

**改动文件**：[src/ui/draw.rs](src/ui/draw.rs)、[src/ui/theme.rs](src/ui/theme.rs)、[Cargo.toml](Cargo.toml)

- 新增依赖：`syntect = { version = "5", default-features = false, features = ["default-fancy"] }`
- draw.rs 新增函数：`syntax_set()` (OnceLock 缓存 SyntaxSet)、`build_theme()` (自定义 Tokyo Night 22 种 scope 配色)、`is_code_fence()` (围栏检测)、`highlight_line()` (syntect → ratatui Span 转换)
- `draw_chat_area()`：消息渲染中检测 ``` 围栏，代码块内容用 HighlightLines 逐行着色
- 代码块视觉：`┌─ rust ──` 语言标签 → 每行 `│ ` 左边框 + 语法高亮 → `└──` 底边框
- 流式 buffer 中未闭合的代码块同样高亮（`stream_code_lang` 状态跟踪）
- theme.rs：移除 `#[allow(dead_code)]`，删除 6 个不再使用的 CODE_* 常量（颜色由 syntect theme 管理）
- 验证：发含 ```rust 代码块的消息，代码应按语法着色（关键字紫色、字符串绿色、函数蓝色）

#### P0-2 输入历史浏览

**改动文件**：[src/app.rs](src/app.rs)

- `AppState` 新增 3 个字段：`input_history: Vec<String>`、`history_idx: Option<usize>`、`input_draft: String`
- Ctrl+Enter 发送后 `state.input_history.push(user_msg.clone())`
- ↑ 箭头：首次按保存当前输入到 `input_draft`，进入历史浏览模式
- ↓ 箭头：到末条后恢复 `input_draft`
- 验证：发送几条消息后按 ↑ 应显示之前发送的内容，按 ↓ 回到最新

---

### Phase 5：P1 核心体验优化（全部完成）

#### P1-5 技能系统增强

**改动文件**：[src/config.rs](src/config.rs)、[src/app.rs](src/app.rs)、[src/main.rs](src/main.rs)、[src/ui/draw.rs](src/ui/draw.rs)

**子任务 1 — 参数传递 `{args}`**
- 改动：Ctrl+Enter 技能执行块（app.rs ~251-259 行）
- 解析：`after_slash.splitn(2, ' ')` 分离技能名和参数
- 替换：命令模板含 `{args}` 则替换，否则追加到末尾
- 示例：`/lint --fix` + 模板 `cargo clippy 2>&1` → `cargo clippy --fix 2>&1`

**子任务 2 — 内置默认技能**
- 改动：config.rs 新增 `apply_defaults()` 方法 + `default_skills()` 函数
- main.rs 调用 `config.apply_defaults()`（加载后、传入 app 前）
- 默认 5 个：lint/test/build/git/diff
- 验证：不配置 skills 时，`/help` 应列出 lint/test/build/git/diff

**子任务 3 — 输出增强**
- 改动：技能执行的 tokio::spawn 块内（app.rs ~283-324 行）
- 计时：`let start = Instant::now();` → 执行 → `elapsed.as_millis()` 显示
- 截断：`result.len() > 2000` 时 `result.truncate(2000)` + `"... (truncated)"`
- 格式：`/lint output (352ms):\n...`

**子任务 4 — 输入提示 + /help**
- 改动 1：Ctrl+Enter 块新增 `/help` 处理（app.rs ~237-250），列出全部排序后的技能名
- 改动 2：新增 `update_hint_lines()` 函数（app.rs ~627-642），输入 `/` 前缀时过滤匹配技能
- 改动 3：各输入变化处（Char/Backspace/Delete/Up/Down）调用 `update_hint_lines(state)`
- 改动 4：draw.rs 新增 `draw_hint_area()` 函数，输入框上方 1 行显示匹配技能名
- 验证：输入 `/l` 应显示 `/lint`，输入 `/help` 应在对话区列出全部技能

#### P1-0 聊天区滚动浏览

**改动文件**：[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)

- `AppState` 新增 `chat_scroll: usize`
- 键盘处理：PageUp → `chat_scroll += 10`，PageDown → `chat_scroll -= 10`，Ctrl+Home → `chat_scroll = 0`
- Done/Error 处理中：新消息到达时 `state.chat_scroll = 0` 自动回底
- `draw_chat_area()` 滚动计算：`let max_scroll = total_lines - visible_height; max_scroll.saturating_sub(state.chat_scroll)`
- 状态栏新增滚动指示器：`[↑N]`（chat_scroll > 0 时显示）
- 验证：多轮对话后按 PageUp 应回看历史，新消息到达自动回底

#### P1-1 消息搜索

**改动文件**：[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)

- `AppState` 新增 4 个字段：`search_active: bool`、`search_query: String`、`search_matches: Vec<usize>`、`search_match_idx: usize`
- Ctrl+F：`search_active = true`，进入搜索模式
- 搜索模式下：Char 追加到 `search_query` + `do_search()`；Backspace 删除末字符 + `do_search()`；Enter 正向跳转；Shift+Enter 反向跳转；Esc 退出
- `do_search()` 函数：遍历 `state.messages`，`content.to_lowercase().contains(&query)` 收集匹配消息 index
- 键盘守卫：搜索模式下，普通 Char/Backspace/Delete/End/Home/箭头 均被 `!state.search_active` 屏蔽
- draw.rs：`draw()` 新增 `search_len` 布局约束；新增 `draw_search_bar()` 显示 `/ query  2/5`
- `draw_chat_area()`：当前匹配消息（`search_matches[search_match_idx]`）背景色高亮 `Color::Rgb(59, 66, 97)`
- `draw_input_area()`：搜索模式下显示 "Searching... (Esc to close)" + "Enter ↵  Esc ✕"
- 验证：Ctrl+F 输入关键词，匹配消息应高亮，Enter 跳转下一条

#### P1-2 启动性能优化

**改动文件**：[src/ui/draw.rs](src/ui/draw.rs)

- 新增 `fn theme() -> &'static Theme`（OnceLock 缓存，只初始化一次）
- `draw_chat_area()` 中 `let theme = build_theme();` 改为 `let syn_theme = theme();`
- 验证：首次 draw 后 theme 不再重复构建

**工程变更**
- Cargo.toml：`syntect = { version = "5", default-features = false, features = ["default-fancy"] }`
- theme.rs：删除 `CODE_BG` / `CODE_KW` / `CODE_STR` / `CODE_FN` / `CODE_COMMENT` / `SELECTED`（6 个未使用常量）
- 编译状态：通过，0 warnings

---

### Phase 6：P1-3 文件操作系统

#### P1-3 文件操作（/read /write /edit）

**改动文件**：新增 [src/file_ops.rs](src/file_ops.rs)、[src/app.rs](src/app.rs)、[src/main.rs](src/main.rs)

**子任务 1 — file_ops.rs 模块（~160 行）**
- `validate_path(path_str, cwd)`：路径安全验证 — 禁止绝对路径、禁止 `../` 穿越工作目录、禁止 `.git/` 写入；Windows `canonicalize` 的 `\\?\` 前缀通过 `strip_unc_prefix()` 自动去除
- `normalize_path(path)`：`canonicalize` + UNC 前缀去除，确保跨平台路径比较一致
- `read_file(path, line_range)`：读取文件，可选行范围 `(start, end)`（1-indexed），带行号前缀 `  42 | ...`；全文件显示总行数 `[N lines]`
- `write_file(path, content)`：写入文件，已存在时自动备份，自动创建父目录
- `backup_file(path)`：备份到 `.mimo-opt/backups/{filename}.{unix_ts}.bak`，清理超过 5 个的旧备份
- `apply_edit(path, old, new)`：精确字符串替换，显示替换次数和新文件大小

**子任务 2 — app.rs 命令集成**
- `handle_read_command(state, args, client, token_tx)`：解析 `path[:start[-end]]` 格式（处理 Windows 盘符 `C:` 冒号冲突）→ 读取文件 → 显示在对话区 → 截断 3000 字符后发送 MiMo 分析
- `handle_write_command(state, args, client, token_tx)`：路径验证 → `extract_last_code_block()` 从对话历史/流式 buffer 提取最后代码块 → 写入文件 → 显示结果
- `handle_edit_command(state, args, client, token_tx)`：`splitn(3, ' ')` 分离 path/old/new → `apply_edit` → 显示结果
- `extract_last_code_block(messages, stream_buffer)`：遍历所有消息从后往前找最后一个 ``` 围栏，支持未闭合的流式代码块
- `extract_code_from_text(text)`：逐行扫描围栏状态机，返回 `(lang, content)`
- `send_to_mimo(state, client, token_tx, analysis_msg)`：抽取的共享逻辑 — 准备消息、应用缓存断点、tokio::spawn 发送

**子任务 3 — 入口注册**
- main.rs：新增 `mod file_ops;`
- app.rs：`use crate::file_ops;`，Ctrl+Enter 块中 `/read` `/write` `/edit` 检查放在 `/help` 之后、技能执行之前
- `update_hint_lines()`：内置命令提示加入 read/write/edit/help
- `/help` 文本更新：显示文件命令 + 技能列表

**安全验证**：
- 验证 1：`/read Cargo.toml` 应显示带行号的文件内容，MiMo 自动分析
- 验证 2：`/read src/main.rs:1-5` 只显示前 5 行
- 验证 3：`/read ../etc/passwd` 应显示 "路径穿越被禁止" 错误
- 验证 4：`/write test.rs` 在有代码块的对话后应写入文件并备份
- 验证 5：`/edit Cargo.toml 0.1.0 0.2.0` 应完成替换并显示备份信息
- 编译状态：通过，0 warnings

---

### Phase 7：P1-4 重要操作确认机制

#### P1-4 确认对话框

**改动文件**：[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)

**子任务 1 — 数据结构**
- `ConfirmAction` 枚举（app.rs）：`WriteFile { path, code, lang }` / `EditFile { path, old, new }` / `ClearChat` / `Quit`
- `ConfirmState` 结构体：`action: ConfirmAction` + `detail: String`（显示文本）
- `AppState` 新增 `pending_confirm: Option<ConfirmState>` 字段

**子任务 2 — 两阶段执行**
- `/write`：Phase 1 验证路径 + `extract_last_code_block()` 提取代码 → 设置 `pending_confirm(WriteFile)`；Phase 2 `execute_confirm()` 调用 `file_ops::write_file()`
- `/edit`：Phase 1 验证路径 + `read_to_string` 预检查匹配 → 计算替换次数 → `pending_confirm(EditFile)`；Phase 2 `execute_confirm()` 调用 `file_ops::apply_edit()`
- `/clear`：新增命令，直接设置 `pending_confirm(ClearChat)`，确认后 `messages.clear()`
- Ctrl+Q：有消息时设置 `pending_confirm(Quit)` 替代直接退出

**子任务 3 — 键盘处理**
- y 键（confirm 活跃时）：调用 `execute_confirm(state, client, token_tx)` 执行待确认操作
- n 键 / Esc（confirm 活跃时）：清除 `pending_confirm`，显示 "已取消"
- d 键（confirm 活跃时）：调用 `show_confirm_detail(state)` 展开详情到对话区
- 确认状态下屏蔽：Ctrl+Enter、Ctrl+F、字符输入（`state.pending_confirm.is_none()` 守卫）

**子任务 4 — UI 渲染**
- `draw_confirm_modal(f, confirm)`：居中弹窗（60×8），`ratatui::widgets::Clear` 清除背景
- 红色圆角边框（`AppTheme::ERROR`），标题行加粗，详情行用 `USER_MSG` 颜色
- 底部操作提示：`y 确认  n 取消  d 详情`

**子任务 5 — /clear + 帮助更新**
- Ctrl+Enter 块新增 `/clear` 分支（在 `/edit` 之后、技能执行之前）
- `/help` 文本更新，标注 "需确认"
- `update_hint_lines()` 内置命令列表新增 "clear"
- Ctrl+Q 改为：有消息 → 显示确认框，无消息或二次确认 → 直接退出

**验证**：
- 验证 1：`/write test.rs`（有代码块时）应弹出红色确认框，按 y 写入，按 n 取消
- 验证 2：`/edit Cargo.toml 0.1.0 0.2.0` 应显示替换次数，按 y 执行
- 验证 3：`/clear` 应显示 "清空 N 条对话"，按 y 清空
- 验证 4：有对话时 Ctrl+Q 应弹出退出确认，按 y 退出
- 验证 5：确认状态下输入字符不应有反应，按 d 应显示详情
- 编译状态：通过，0 warnings

---

### Phase 8：P1-6 Content enum 改造

#### P1-6 Content enum

**改动文件**：[src/api/types.rs](src/api/types.rs)、[src/api/mod.rs](src/api/mod.rs)、[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)

**types.rs 改动**：
- 新增 `Content` 枚举：`#[serde(untagged)] pub enum Content { Text(String) }`
- 辅助方法：`Content::text(s)` 构造 / `as_str()` 取引用 / `as_mut_str()` 取可变引用
- `ChatMessage.content` 类型从 `String` 改为 `Content`
- `#[serde(untagged)]` 保证 API 兼容：`Content::Text("hello")` 序列化为 `"hello"`

**app.rs 迁移**：
- 全部 15 处 `ChatMessage` 创建：`content: "...".to_string()` → `content: Content::text("...")`
- 2 处 `state.messages[0].content.push_str(...)` → `state.messages[0].content.as_mut_str().push_str(...)`
- 1 处搜索 `msg.content.to_lowercase()` → `msg.content.as_str().to_lowercase()`
- 1 处代码提取 `&msg.content` → `msg.content.as_str()`

**draw.rs 迁移**：
- 1 处渲染 `msg.content.lines()` → `msg.content.as_str().lines()`

**api/mod.rs**：
- 1 处 `check_api` 的测试消息 `content: "hi".to_string()` → `content: Content::text("hi")`

**为未来预留**：新增 `Image` / `ToolUse` / `ToolResult` 变体只需加枚举成员，不改现有代码。
- 编译状态：通过，0 warnings

---

### Phase 9：P3 附加功能

#### P3-9 代码块复制

**改动文件**：[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)、[Cargo.toml](Cargo.toml)

- 新增依赖：`cli-clipboard = "0.4"`
- `AppState` 新增字段：`code_blocks: Vec<(String, String)>`（代码块列表）、`copy_status: Option<String>`（复制反馈）
- `collect_code_blocks(state)`（app.rs ~812-840）：遍历所有消息，逐行扫描 ``` 围栏，收集 (lang, content) 对
- Ctrl+Y 处理（app.rs ~289-301）：调用 `collect_code_blocks` → 取最后一个代码块 → `cli_clipboard::set_contents()` → 设置 copy_status
- `draw_status_bar()`（draw.rs）：copy_status 非空时显示 "✓ 已复制 rust 代码块 (12 行)"
- 按键清除：任意按键时 `state.copy_status = None`，状态一闪即逝
- 验证：发送含代码块的消息，按 Ctrl+Y 应复制代码到剪贴板，状态栏显示确认
- 编译状态：通过，0 warnings

#### P3-10 技能管理

**改动文件**：[src/app.rs](src/app.rs)、[src/config.rs](src/config.rs)

- `Config` 新增 `save()` 方法（config.rs）：序列化为 JSON 写入配置文件
- `/skills` 命令：格式化列出全部技能（名称 → 命令模板），底部提示 /addskill /rmskill
- `/addskill <name> <cmd>`：`splitn(2, ' ')` 分离名称和命令 → `state.skills.insert()` → `config.save()` → 显示确认
- `/rmskill <name>`：`state.skills.remove()` → `config.save()` → 显示确认；不存在时报错
- `/help` 更新：新增 skills / addskill / rmskill 说明
- `update_hint_lines()` 内置命令列表新增 skills / addskill / rmskill
- 验证：`/addskill lint "cargo clippy 2>&1"` 应添加技能并写入 config.json，重启后仍存在
- 验证：`/rmskill lint` 应删除技能，`/skills` 不再列出
- 编译状态：通过，0 warnings

---

### Phase 10：P2-7 多会话持久化

#### P2-7 多会话

**改动文件**：新增 [src/session.rs](src/session.rs)、[src/app.rs](src/app.rs)、[src/ui/draw.rs](src/ui/draw.rs)、[src/main.rs](src/main.rs)

**session.rs 模块（~110 行）**：
- `Session` 结构体：`id: String`（时间戳）+ `name` + `messages: Vec<ChatMessage>` + `created_at` + `updated_at`
- `Session::new(name)`：创建新会话
- `Session::auto_name()`：生成 `session-{days}_{HHmm}` 格式名称
- `Session::save()`：序列化为 JSON 写入 `~/.config/mimo-opt/sessions/{id}.json`
- `Session::load(id)`：从 JSON 文件反序列化
- `Session::list()`：扫描 sessions 目录，按 `updated_at` 降序返回 `Vec<SessionInfo>`

**app.rs 集成**：
- `AppState` 新增 `session: Session` + `session_list: Vec<SessionInfo>`
- 启动时加载最近会话：`Session::list().first()` → `Session::load()` → 恢复 messages
- Ctrl+N：保存当前会话 → `Session::new(auto_name())` → 清空 messages/counters
- F2：保存当前 → 循环切换到下一个会话 → 恢复 messages
- `StreamResult::Done` 中：每 5 条消息自动保存一次
- Ctrl+Q 退出（直接 + 确认两条路径）均自动保存

**draw.rs 标题栏**：
- `MiMo-OPT` 后追加 `[session_name]`，超 20 字符截断

**main.rs**：新增 `mod session;`

**验证**：
- 验证 1：发送几条消息后 Ctrl+Q 退出，重启应恢复对话
- 验证 2：Ctrl+N 应清空对话，标题栏显示新会话名
- 验证 3：F2 应切换到另一个会话，对话内容不同
- 验证 4：`~/.config/mimo-opt/sessions/` 下应有 JSON 文件
- 编译状态：通过，0 warnings

---

## 下一步计划

- [ ] **P2-8** 桌面端迁移（Tauri）— 已决定终端版先交付检查

---

## 2026-05-19（续）

### v0.2.0 — 代码去重 + API 多格式架构预留

**里程碑**：首个具备多 API 格式扩展能力的稳定版本。

**已实现功能**：流式对话 / 多会话持久化 / 文件操作（/read /write /edit）/ 技能系统 / 代码语法高亮 / 消息搜索 / 输入历史 / 剪贴板复制 / 确认对话框 / Anthropic + OpenAI 双格式架构 / Tokyo Night 配色

### Phase 11：代码去重 + API 多格式架构预留

#### 11-1 新增 `src/util.rs` 共享工具模块

**新增文件**：[src/util.rs](src/util.rs)

- `now_secs()` — Unix 时间戳（秒），统一取代散落各文件的 5 处内联实现
- `get_cwd()` — 获取当前工作目录，统一错误信息格式

**改动文件**：[src/main.rs](src/main.rs) — 新增 `mod util;`

#### 11-2 时间戳去重

**改动文件**：[src/session.rs](src/session.rs)、[src/file_ops.rs](src/file_ops.rs)、[src/app.rs](src/app.rs)

- session.rs：删除本地 `now_secs()`，改为 `use crate::util::now_secs`
- file_ops.rs `backup_file()`：内联 3 行 → `now_secs()`
- app.rs `chrono_date()`：内联 4 行 → `now_secs()`
- 消除 5 处重复的时间戳生成代码

#### 11-3 会话保存去重（5 → 1）

**改动文件**：[src/app.rs](src/app.rs)

- 新增 `save_session(state)` 函数：`state.session.messages = state.messages.clone(); state.session.updated_at = now_secs(); state.session.save()`
- 替换 5 处相同代码块：
  1. `StreamResult::Done` 自动保存（每 5 条）
  2. Ctrl+N 新建会话前
  3. F2 切换会话前
  4. Ctrl+Q 直接退出
  5. `execute_confirm(Quit)` 确认退出

#### 11-4 路径验证去重（5 → 1）

**改动文件**：[src/app.rs](src/app.rs)

- 5 处 `std::env::current_dir()` + 错误处理 → `get_cwd()` 统一调用
- `handle_read_command`、`handle_write_command`、`handle_edit_command`、`execute_confirm`(WriteFile/EditFile)、`scan_project_tree`

#### 11-5 取消消息去重

**改动文件**：[src/app.rs](src/app.rs)

- 新增 `push_cancelled_message(state)` 函数
- n 键和 Esc 取消确认时统一调用（之前各有一份相同的 ChatMessage 创建代码）

#### 11-6 API 多格式架构（Anthropic + OpenAI 兼容）

**改动文件**：[src/api/types.rs](src/api/types.rs)、[src/api/mod.rs](src/api/mod.rs)、[src/config.rs](src/config.rs)、[src/app.rs](src/app.rs)、[src/main.rs](src/main.rs)

**types.rs 改动**：
- `ChatRequest` → `AnthropicRequest`（Anthropic 格式专用）
- 新增 `OpenAIRequest`、`OpenAIMessage`、`OpenAIStreamEvent`、`OpenAIChoice`、`OpenAIDelta`（OpenAI 兼容格式）
- `StreamEvent` → `AnthropicStreamEvent`，`Delta` → `AnthropicDelta`，`StreamMessage` → `AnthropicStreamMessage`
- `SystemContent` 新增 `Clone` derive

**config.rs 改动**：
- `Config` 新增 `api_format: String`（`"anthropic"` | `"openai"`，默认 `"anthropic"`）
- `Config` 新增 `max_tokens: u32`（默认 `4096`）
- `ConfigRaw` 同步新增两个可选字段

**api/mod.rs 改动**：
- `MiMoClient` 新增 `api_format` 和 `max_tokens` 字段
- `new()` 根据 `api_format` 自动选择端点路径：`/v1/messages`（Anthropic）或 `/v1/chat/completions`（OpenAI）
- 请求构建分流：`build_anthropic_request()` / `build_openai_request()`
- 响应解析分流：`stream_anthropic()` / `stream_openai()`
- OpenAI 格式将 `ChatMessage` 转换为 `OpenAIMessage`（忽略 cache_control），`[DONE]` 事件触发 `StreamResult::Done`
- Anthropic 格式逻辑不变

**设计原则**：
- 认证方式（`auth_type`）和数据格式（`api_format`）正交：可组合为 anthropic+anthropic、bearer+openai 等
- 两套流式解析器独立实现，不混用，避免兼容性问题
- 默认值（MiMo Token Plan）完全不变，零配置向后兼容
- 预留空间：未来新增格式只需加 `build_xxx_request()` + `stream_xxx()` 方法对

**main.rs 改动**：
- 首次运行提示新增"方案三"：第三方 OpenAI 兼容 API（DeepSeek、GLM 等）配置示例
- MiMoClient::new() 调用处传递 `api_format` 和 `max_tokens`

**编译状态**：通过，0 warnings

**回滚说明**：
- 如需回滚 API 格式改动：将 `api/types.rs` 中 `AnthropicRequest` 改回 `ChatRequest`，删除 OpenAI 相关类型，`api/mod.rs` 恢复为单格式实现
- 如需回滚去重改动：将 `save_session` / `push_cancelled_message` / `get_cwd` 调用处还原为内联代码，删除 `src/util.rs`
- Config 新增的 `api_format` 和 `max_tokens` 字段为 `Option`，旧配置文件无需迁移
