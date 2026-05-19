# MiMo-OPT 更新日志

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

- [ ] **P2-8** 桌面端迁移（Tauri）— 终端版先交付
- [ ] 第三方 API 逐个实测验证（DeepSeek / GLM / 通义千问）
