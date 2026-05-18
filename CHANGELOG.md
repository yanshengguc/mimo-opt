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

---

## 下一步计划

- [ ] 代码块语法高亮（syntect）
- [ ] 多会话 + 持久化
- [ ] 项目上下文感知（@文件引用）
- [ ] Shell 命令集成
- [ ] 模型路由（flash/pro/auto）
- [ ] 前缀缓存优化
