# MiMo-OPT — 项目需求文档

> 小米 MiMo Token Plan API 的终端聊天工具。目标用户是程序员。

---

## 项目目标

打造一款极简、高效的 MiMo API 终端客户端，程序员接上 API Key 就能用。

**核心卖点：**
1. 缓存优化直接省钱（70%+ 命中率）
2. 多 Provider 支持（MiMo / DeepSeek / OpenAI / 自定义）
3. 代码块语法高亮（syntect）
4. 文件操作（read/write/edit）
5. 技能系统（/命令执行 shell）
6. 多会话持久化
7. 单二进制零依赖部署

---

## 技术栈

| 层 | 技术 |
|---|------|
| 语言 | Rust 2021 |
| 终端 UI | ratatui 0.29 + crossterm 0.28 |
| HTTP | reqwest 0.12 (json, stream) |
| 异步 | tokio 1 (full) |
| 序列化 | serde + serde_json |
| 语法高亮 | syntect 5 |
| 剪贴板 | arboard 3 |
| 其他 | scopeguard, unicode-width, futures-util, dirs, anyhow |

---

## 功能清单

### 已完成（终端版）

| 类别 | 功能 | 状态 |
|------|------|------|
| **核心** | MiMo API 流式对话（SSE） | ✅ |
| | 双 API 认证（anthropic / bearer） | ✅ |
| | ratatui 终端界面（Tokyo Night 配色） | ✅ |
| | UTF-8 跨 chunk 安全解码 | ✅ |
| | API 状态探测 + 错误传递 | ✅ |
| | token 用量追踪 + 费用计算 | ✅ |
| **缓存** | 渐进式多断点缓存策略 | ✅ |
| | System prompt 去日期化 | ✅ |
| | 缓存命中率实时显示 | ✅ |
| **代码块** | syntect 语法高亮 | ✅ |
| | 流式未闭合代码块高亮 | ✅ |
| | 代码块复制（Ctrl+Y） | ✅ |
| **交互** | 输入历史浏览（↑/↓） | ✅ |
| | 聊天区滚动（PageUp/Down） | ✅ |
| | 消息搜索（Ctrl+F） | ✅ |
| | Ctrl+V 粘贴剪贴板 | ✅ |
| | 代码块深色背景 #1a1b26 | ✅ |
| **文件操作** | /read 读取文件 | ✅ |
| | /write 写入文件 | ✅ |
| | /edit 字符串替换 | ✅ |
| | 路径沙箱 + 自动备份 | ✅ |
| **技能系统** | /命令执行 shell | ✅ |
| | 参数传递 `{args}` | ✅ |
| | 5 个内置默认技能 | ✅ |
| | 输入提示 + /help | ✅ |
| | /skills /addskill /rmskill 管理 | ✅ |
| **会话** | 多会话持久化 | ✅ |
| | Ctrl+N 新建 / F2 切换 | ✅ |
| | 自动保存（每 5 条 + 退出） | ✅ |
| **安全** | 破坏性操作确认弹窗 | ✅ |
| | Content enum 前向兼容 | ✅ |
| **热切换** | /model 运行时切换模型 | ✅ |
| | /provider 切换 API 提供商 | ✅ |
| **会话** | token/费用持久化（重启恢复） | ✅ |
| **UX** | 长消息发送确认（>5 行） | ✅ |
| | 对话轮次分隔线 | ✅ |
| | 终端最小尺寸检查 | ✅ |
| | 错误历史 + /errors 查看 | ✅ |
| **导出** | /export 会话导出 Markdown | ✅ |
| **日志** | 结构化日志（log+env_logger） | ✅ v0.3.7 |
| **重试** | API 自动重试（5xx/429/网络） | ✅ v0.3.7 |
| **余额** | DeepSeek 余额查询 | ✅ v0.3.7 |
| **余额增强** | 货币单位 + 颜色预警 | ✅ v0.3.8 |
| **会话名** | 目录名友好命名 | ✅ v0.3.8 |
| **退出** | 退出用量汇总打印 | ✅ v0.3.8 |
| **引用块** | `>` 竖线+缩进渲染 | ✅ v0.3.9 |
| **Markdown** | 粗体/斜体/行内代码 | ✅ v0.3.9 |
| **Markdown 完善** | 列表/链接/分隔线 | ✅ v0.4.0 |
| **主题** | 3 套主题热切换 + /theme | ✅ v0.4.0 |
| **缓存 v3** | 日期前置+System Prompt分割+自适应断点 | ✅ v0.5.0 |
| **代理** | HTTP/SOCKS5 代理支持 | ✅ v0.5.0 |
| **联网搜索** | /search DDG+DeepSeek 双引擎 | ✅ v0.5.0 |
| **费用预估** | 发送前实时 token/费用预估 | ✅ v0.5.0 |
| **撤回** | Ctrl+Z 撤回最后对话轮次 | ✅ v0.5.0 |
| **输入框** | 自适应多行扩展 (3~半屏) | ✅ v0.5.0 |
| **异步加载** | syntect 异步预加载不阻塞首屏 | ✅ v0.5.0 |
| **diff 预览** | /edit 确认弹窗红删绿增 | ✅ v0.5.0 |
| **通知** | 回复完成终端响铃 | ✅ v0.5.0 |
| **Logo** | 芒果猫 ANSI 色块启动动画 | ✅ v0.5.0 |
| **剪贴板** | cli-clipboard → arboard 跨平台 | ✅ v0.5.0 |
| **多模型支持** | 每 Provider 多个模型 + 独立定价 + 补全提示 | ✅ v0.5.2 |

### 待完成

| 功能 | 优先级 | 说明 |
|------|--------|------|
| 单元测试覆盖 | P2 | file_ops / config / prompt / cost / session |
| app.rs 模块拆分 | P2 | ~1900 行 → 多文件 |
| 会话侧边栏 (Ctrl+B) | P3 | UI_DESIGN 已规划 |
| Shell 管道集成 | P3 | `echo "..." \| mimo-opt --prompt` |
| 桌面端（Tauri） | P2-8 | 已决定终端版先交付 |
| MCP 支持 | P3 | |

---

## 项目快照

- **当前行数**：~3700 行 Rust（16 个源文件）
- **依赖**：tokio, reqwest, serde, serde_json, ratatui, crossterm, futures-util, dirs, anyhow, scopeguard, unicode-width, syntect, arboard, log, env_logger
- **编译状态**：通过，0 warnings，clippy clean
- **API 端点**：MiMo Token Plan / DeepSeek / OpenAI / 自定义（多 Provider 预设）
- **默认模型**：mimo-v2-flash
- **Provider**：`/model` 和 `/provider` 运行时热切换

---

## 关键文档

| 文档 | 说明 |
|------|------|
| [OPTIMIZATION.md](OPTIMIZATION.md) | 完整优化路线图（15 项已完成 + 待做方向） |
| [CHANGELOG.md](CHANGELOG.md) | 详细更新日志（Phase 1-10） |
| [HANDOFF.md](HANDOFF.md) | AI 交接文档（项目全貌 + 实现细节） |
| [README.md](README.md) | GitHub 项目介绍 |
| [UI_DESIGN.md](UI_DESIGN.md) | 界面视觉设计稿 |
