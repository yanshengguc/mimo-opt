# MiMo-OPT

> **芒果猫** · 终端 AI 编程助手 · v0.5.0

Rust 编写的多 Provider 终端 AI 聊天工具。接上 API Key 就能用，单二进制 ~5MB，零运行时依赖。

支持 **DeepSeek / MiMo / OpenAI / 自定义 OpenAI 兼容 API**，开箱即用。

> 项目仍在积极开发中，后续更新桌面端（Tauri）。遇到 Bug 或有优化建议，欢迎联系作者 QQ：**2391859666**

## 特性

### 多 Provider 支持

4 种预设 + 自定义 Open​AI 兼容端点，`/provider` 命令运行时热切换，无须重启：

| Provider | API 格式 | 状态 |
|----------|----------|------|
| DeepSeek | OpenAI 兼容 (bearer) | ✅ 默认 |
| MiMo Token Plan | Anthropic | ✅ 已集成 |
| OpenAI | OpenAI 兼容 (bearer) | ✅ 已集成 |
| 自定义 | OpenAI 兼容 | ✅ 任意兼容 API |

### 缓存优化 — 省 85-92% 输入费用

Anthropic 格式完整缓存方案（v3），状态栏实时显示 `♻` 命中率：

- **System Prompt 拆分**：身份+格式规则稳定缓存，cwd/项目文件树独立 block 不污染
- **日期移出缓存前缀**：日期以独立 preamble 消息注入，不破坏 stable block 缓存
- **自适应多断点**：按对话长度动态分配 4 个断点（1-3/4-8/9-16/17+），末轮对话始终命中

### 代码块语法高亮

`syntect` + Tokyo Night 配色，100+ 语言，流式输出中未闭合代码块也实时着色。深色背景 + 语言标签边框。

### Markdown 全覆盖

粗体 / 斜体 / 行内代码 / 引用块（`│` 竖线）/ 有序&无序列表 / 链接 / 水平分隔线 — 七种元素全部渲染。

### 文件操作

AI 直接读/写/编辑项目文件：

- `/read src/main.rs:42` — 读取文件注入上下文（带行号，支持行范围）
- `/write src/foo.rs` — 提取对话中最后一个代码块写入文件
- `/edit src/main.rs old_str new_str` — 精确字符串替换
- 所有写入操作自动备份（5 版本轮转），路径沙箱防穿越和 `.git/` 写入

### 项目感知

启动时自动扫描当前目录（最多 100 个文件，自动排除 `node_modules`/`target`/`.git` 等），注入 system prompt。AI 开箱就知道你的项目结构。

### 技能系统

`/命令名` 执行预配置 shell 命令，输出自动发送给 AI 分析。支持参数传递、5 个内置默认技能、TUI 内增删技能。

### 联网搜索

`/search <关键词>` 触发，DDG 免费 API（无需 Key）+ DeepSeek 原生 web_search 双引擎。搜索结果格式化注入对话，AI 可基于实时信息回答问题。

### 智能输入体验

- **费用预估**：输入框右侧实时显示 `~¥0.02 ~800tok`，超 20000tok 或 ¥0.5 红色预警
- **自适应输入框**：内容超 3 行自动扩展（上限半屏），中文宽度精确感知
- **Ctrl+Z 撤回**：误发消息后移除最后对话轮次，消息恢复到输入框
- **编辑重发**：↑/↓ 浏览已发送消息，编辑后 Ctrl+Enter 重发
- **Ctrl+V 粘贴** / **Ctrl+Y 复制代码块**：Wayland/X11/macOS/Windows 全平台支持（arboard）

### 芒果猫启动 Logo

ANSI 真彩色块像素画（8×5 猫脸），橘色系配色，右侧展示版本/Provider/Model/余额。按任意键进入主界面。

### 3 套内置主题

`/theme tokyo-night|nord|catppuccin` 即时切换，无需重启，选择持久化到 config.json。

| 主题 | 风格 |
|------|------|
| Tokyo Night (默认) | 紫蓝暗色 |
| Nord | 蓝灰冷色 |
| Catppuccin | 柔和暖色 |

### 多会话管理

会话自动保存到 `~/.config/mimo-opt/sessions/`。`Ctrl+N` 新建、`F2` 切换。会话名自动生成为 `{目录名}_{HHMM}` 格式，可辨识。

### 成本追踪 & 余额

状态栏实时显示：累计 token 数（K）、输入/输出分别计数（`↓↑`）、缓存命中率（`♻`）、费用（`¥`）。

DeepSeek Provider 启动时自动查询余额，标题栏显示 `¥X.XX`，余额不足时颜色预警（红 < ¥1，黄 < ¥5，绿 ≥ ¥5）。

退出时打印用量汇总：消息数 / token / 命中率 / 总费用。

### 其他

- **HTTP/SOCKS5 代理**：`proxy_url` 配置，国内网络兜底
- **temperature / top_p**：config 可选透传，控制模型输出创造性
- **回复通知**：AI 回复完成终端响铃 `\x07`
- **/edit diff 预览**：确认弹窗红删绿增，心里有底再写入
- **syntect 异步加载**：首屏不阻塞，启动秒开
- **UTF-8 安全**：正确处理跨 chunk 多字节字符，中文/emoji 不断裂
- **单二进制**：~5MB，SSH 到远程服务器直接用
- **0 clippy warnings**：代码质量基线
- **Ctrl+F 搜索**、**长消息确认**、**终端尺寸检查**

## 快速开始

### 1. 下载

前往 [Releases](https://github.com/yanshengguc/mimo-opt/releases) 下载预编译版本：

| 平台 | 文件 |
|------|------|
| Windows x64 | `mimo-opt-x86_64-pc-windows-msvc.zip` |
| Linux x64 | `mimo-opt-x86_64-unknown-linux-gnu.tar.gz` |
| macOS ARM | `mimo-opt-aarch64-apple-darwin.tar.gz` |
| macOS x64 | `mimo-opt-x86_64-apple-darwin.tar.gz` |

或从源码构建：

```bash
git clone https://github.com/yanshengguc/mimo-opt.git
cd mimo-opt
cargo build --release
```

### 2. 获取 API Key

任选一个 Provider：

- [DeepSeek](https://platform.deepseek.com/api_keys) — 推荐，国内注册方便，有免费额度
- [小米 MiMo](https://platform.xiaomimimo.com/#/console/subscription) — Token Plan 或普通 API
- [OpenAI](https://platform.openai.com/api-keys) — GPT-4o 系列

### 3. 首次运行

```bash
./target/release/mimo-opt
```

首次运行无配置时会展示 5 种 Provider 的配置引导，编辑 `~/.config/mimo-opt/config.json` 填入密钥后重新运行即可。

### 4. 配置参考

配置文件位于 `~/.config/mimo-opt/config.json`（Windows: `%APPDATA%\mimo-opt\config.json`）。

**DeepSeek（推荐，默认）**：

```json
{
  "provider": "deepseek",
  "api_key": "sk-你的密钥",
  "base_url": "https://api.deepseek.com",
  "model": "deepseek-chat",
  "auth_type": "bearer",
  "api_format": "openai",
  "max_tokens": 4096,
  "theme": "tokyo-night",
  "proxy_url": "http://127.0.0.1:7890",
  "temperature": 0.7,
  "web_search": { "enabled": true, "engine": "ddg", "max_results": 5 },
  "skills": {
    "lint": "cargo clippy 2>&1",
    "test": "cargo test 2>&1"
  }
}
```

**MiMo Token Plan（Anthropic 格式，支持缓存）**：

```json
{
  "provider": "mimo",
  "api_key": "tp-你的密钥",
  "base_url": "https://token-plan-sgp.xiaomimimo.com/anthropic",
  "model": "mimo-v2-flash",
  "auth_type": "anthropic",
  "api_format": "anthropic",
  "max_tokens": 4096
}
```

**OpenAI**：

```json
{
  "provider": "openai",
  "api_key": "sk-你的密钥",
  "base_url": "https://api.openai.com",
  "model": "gpt-4o-mini",
  "auth_type": "bearer",
  "api_format": "openai",
  "max_tokens": 4096
}
```

> 可选字段：`proxy_url`（HTTP/SOCKS5 代理）、`temperature`/`top_p`（推理参数）、`theme`（tokyo-night/nord/catppuccin）、`web_search`（联网搜索配置）、`skills`（自定义技能命令）。

**自定义 OpenAI 兼容 API**（GLM、通义千问等）：

```json
{
  "provider": "custom",
  "api_key": "你的密钥",
  "base_url": "https://api.example.com",
  "model": "your-model",
  "auth_type": "bearer",
  "api_format": "openai",
  "max_tokens": 4096
}
```

### 配置字段说明

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `provider` | `"deepseek"` | Provider 预设名称：`deepseek` / `mimo` / `openai` / `custom` |
| `api_key` | (空) | API 密钥 |
| `base_url` | — | API 端点地址 |
| `model` | — | 模型名称 |
| `auth_type` | — | 认证方式：`anthropic`（x-api-key header）或 `bearer`（Authorization: Bearer） |
| `api_format` | — | 数据格式：`anthropic`（Anthropic Messages API）或 `openai`（OpenAI Chat Completions） |
| `max_tokens` | `4096` | 单次最大输出 token 数 |
| `theme` | `"tokyo-night"` | UI 主题：`tokyo-night` / `nord` / `catppuccin` |
| `skills` | 5 个内置默认 | 技能命令映射，如 `{"lint": "cargo clippy 2>&1"}` |

> `auth_type` 和 `api_format` 正交组合，适配各 Provider 的不同 API 风格。

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Enter` | 发送消息 / 执行技能 |
| `Ctrl+C` / `Esc` | 中断生成（不退出） |
| `Ctrl+Q` | 退出（有对话时弹确认框） |
| `Ctrl+N` | 新建会话 |
| `F2` | 切换会话 |
| `Ctrl+F` | 搜索对话中的消息 |
| `Ctrl+Y` | 复制最后一个代码块到剪贴板 |
| `Ctrl+V` | 粘贴剪贴板内容 |
| `↑/↓` | 浏览输入历史 |
| `←/→` / `Home/End` | 光标移动（UTF-8 安全） |
| `Ctrl+A/Ctrl+E` | 光标行首/行尾 |
| `PageUp/PageDown` | 翻页浏览对话 |
| `Ctrl+Home` | 回到对话底部 |

## 内置命令

| 命令 | 功能 |
|------|------|
| `/read <path>[:range]` | 读取文件注入上下文（如 `/read src/main.rs:10-50`） |
| `/write <path>` | 提取对话中最后一个代码块写入文件（需确认） |
| `/edit <path> <old> <new>` | 精确字符串替换（需确认，自动备份） |
| `/export [path]` | 导出会话为 Markdown（含元信息，默认 `{session}.md`） |
| `/clear` | 清空当前对话（需确认） |
| `/model` | 列出当前 Provider 所有模型 + 描述 + 定价 |
| `/model <name>` | 切换模型（如 `/model deepseek-v4-pro`），含定价确认 |
| `/provider` | 列出所有 Provider 预设 |
| `/provider <name>` | 切换 Provider（如 `/provider openai`），智能保留兼容模型 |
| `/search <关键词>` | 联网搜索（DDG 免费引擎 / DeepSeek 原生） |
| `/theme` | 列出所有可用主题 |
| `/theme <name>` | 切换主题（`tokyo-night` / `nord` / `catppuccin`），立即生效 |
| `/skills` | 列出全部技能 |
| `/addskill <name> <cmd>` | 添加技能 |
| `/rmskill <name>` | 删除技能 |
| `/errors` | 查看 API 错误历史（最近 20 条） |
| `/export [path]` | 导出会话为 Markdown |
| `/clear` | 清空当前对话（需确认） |
| `/read <path>[:range]` | 读取文件注入上下文（如 `/read src/main.rs:10-50`） |
| `/write <path>` | 提取对话中最后一个代码块写入文件（需确认） |
| `/edit <path> <old> <new>` | 精确字符串替换（需确认，自动备份） |
| `/help` | 显示帮助 |
| `/skill_name [args]` | 执行技能命令（如 `/lint --fix`） |

## 技能系统

输入 `/命令名` 执行预配置 shell 命令，输出自动显示在对话区并发送给 AI 分析。

配置示例（`config.json` 的 `skills` 字段）：

```json
"skills": {
  "lint": "cargo clippy 2>&1",
  "test": "cargo test 2>&1",
  "git": "git log --oneline -10",
  "diff": "git diff",
  "build": "cargo build 2>&1"
}
```

- 输入 `/lint` → 执行 `cargo clippy 2>&1`，输出自动发给 AI 分析
- 输入 `/lint --fix` → 参数 `--fix` 追加到命令末尾
- 不配置 `skills` 时自动提供 5 个内置默认：`lint` / `test` / `build` / `git` / `diff`
- `/addskill <name> <cmd>` 和 `/rmskill <name>` 在 TUI 内管理，自动保存配置

跨平台：Windows 使用 `cmd /C`，其他平台使用 `sh -c`。命令输出经 `shell_escape()` 防护注入。

## 提示词缓存策略

仅在 Anthropic 格式（MiMo Token Plan）下生效。

```
请求 1: [System ●]  ← 缓存 system prompt
请求 2: [System] [msg0 ●] [msg1] [msg2] [msg3]  ← system 命中，msg0 新建缓存
请求 3: [System] [msg0] [msg1] [msg2] [msg3] [msg4 ●] ...  ← system+msg0 命中，msg4 新建
...
请求 N: [System] [msg0] [msg1] [msg2 ●] ... [msgN-2 ●] [msgN-1] [msgN]  ← 多断点命中
```

- System prompt 带 `cache_control: ephemeral`（稳定身份+规则部分）
- 第一个 user message 带缓存断点
- 每 5 条消息追加一个断点，最多 4 个（Anthropic 限制）
- 日期通过 uncached preamble 注入，不破坏缓存前缀（v3 计划）
- 状态栏 `♻ XX%` 实时显示命中率

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust 2021 edition |
| TUI 框架 | ratatui 0.29 + crossterm 0.28 |
| 异步运行时 | tokio (full features) |
| HTTP 客户端 | reqwest (json + stream) |
| 序列化 | serde + serde_json |
| 语法高亮 | syntect 5 (default-fancy) |
| 剪贴板 | cli-clipboard 0.4 |
| 日志 | log + env_logger |
| 终端安全 | scopeguard |
| Unicode 宽度 | unicode-width 0.2 |

## 项目结构

```
src/
├── main.rs           # 入口，首次运行引导 5 种 Provider 方案
├── config.rs         # 配置管理（Config/ProviderPreset/WebSearchConfig，JSON 读写 0600）
├── app.rs            # AppState + 事件循环 + 键盘分发 + Logo 展示
├── commands.rs       # 命令调度 + 文件操作 + 确认弹窗 + 搜索 + 技能 + /search
├── prompt.rs         # System prompt 构建 + 缓存断点 v3 + 日期 preamble
├── scanner.rs        # 项目文件树扫描
├── search.rs         # DDG/DeepSeek 联网搜索 + HTML 解析
├── file_ops.rs       # 文件 read/write/edit + 路径沙箱 + 自动备份 + diff 预览
├── session.rs        # 会话持久化（多会话 JSON 存储 + 自动命名）
├── util.rs           # now_secs() / get_cwd() / estimate_tokens()
├── api/
│   ├── mod.rs        # MiMoClient（流式 SSE + 重试 + RwLock 热切换 + web_search）
│   └── types.rs      # API 类型（Content enum + Anthropic/OpenAI 双格式 + temperature/top_p）
└── ui/
    ├── mod.rs        # 模块导出
    ├── logo.rs       # 芒果猫 ANSI 色块启动 Logo
    ├── theme.rs      # ThemeColors + 3 套色板
    └── draw.rs       # 界面渲染（自适应布局 + Markdown + 高亮 + diff + 费用预估）
```

## 评分 & 路线图

当前评分 **~8.3/10**（详见 [OPTIMIZATION.md](OPTIMIZATION.md)）。

| 维度 | 分数 |
|------|------|
| 安全性 | 8.5 |
| 稳定性 | 8.0 |
| 代码质量 | 7.5 |
| 测试覆盖 | 0.5 |
| UI/UX | 8.5 |
| 功能完整度 | 8.0 |
| 文档质量 | 9.0 |
| 工程化 | 8.0 |

后续路线：缓存 v3（85-92% 命中率）→ 代理/编辑重发/费用预估/撤回等用户视角优化 → 联网搜索 → 桌面端（Tauri）。目标 v1.0 评分 9.0。

## 与其他工具的对比

| 特性 | MiMo-OPT | 通用 API 终端工具 | 浏览器聊天 |
|------|----------|-------------------|------------|
| 缓存命中优化 | 前缀缓存 + 多断点策略 | 无 | 取决于平台 |
| 缓存命中率可视化 | `♻` 状态栏实时显示 | ❌ | ❌ |
| 代码块语法高亮 | syntect 100+ 语言 | 部分 | ✅ |
| Markdown 渲染 | 7 种元素全覆盖 | 少 | ✅ |
| 项目文件感知 | 自动扫描注入 | ❌ | ❌ |
| 文件操作 | read/write/edit + 自动备份 | ❌ | ❌ |
| 技能系统 | /命令 → AI 分析 | ❌ | ❌ |
| 多 Provider | DeepSeek/MiMo/OpenAI/自定义 | 需手动配置 | 取决于平台 |
| Provider 热切换 | `/provider` 即时切换 | ❌ | ❌ |
| 主题热切换 | 3 套内置 + `/theme` | 少 | 部分 |
| 余额查询 & 预警 | DeepSeek 自动查询 + 三色预警 | ❌ | 部分平台 |
| 多会话管理 | 自动保存 + 命名 + 切换 | 部分 | ✅ |
| 流式 UTF-8 安全 | 跨 chunk 处理 | 部分 | ✅ |
| 单二进制部署 | ~5MB | 依赖运行时 | 不适用 |
| 确认机制 | 破坏性操作需确认 | 无 | 无 |
| 0 clippy warnings | ✅ | 不保证 | 不适用 |

## License

MIT
