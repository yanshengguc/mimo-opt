# MiMo-OPT

小米 MiMo API 终端聊天工具 —— Rust 编写的极简客户端，接上 API Key 就能用。

> **测试阶段** — 项目仍在积极开发中目前只有控制台终端可用，后续更新桌面端，功能和接口可能变动。遇到 Bug 或有优化建议，欢迎联系作者 QQ：**2391859666**

## 为什么选 MiMo-OPT

### 缓存优化 — 直接省钱

大多数终端聊天工具每次请求都发送完整上下文，API 端全部重新计算。MiMo-OPT 实现了 Anthropic Messages API 的前缀缓存机制：

- **System prompt 跨天缓存**：System prompt 不含日期，缓存跨天存活，避免每日重建开销
- **渐进式多断点**：`messages[0]` + 每 6 条消息一个断点，长对话命中率稳定 70%+
- **状态栏实时显示** `♻ XX%` 缓存命中率，花了多少钱一目了然

### 代码块语法高亮

使用 `syntect` 实现 Tokyo Night 配色的语法高亮，支持 100+ 种语言。代码块有深色背景、语言标签边框，流式输出中未闭合的代码块同样着色。

### 文件操作

MiMo 直接读/写/编辑项目文件，从"聊天工具"升级为"编码助手"：

- `/read src/main.rs:42` — 读取文件注入上下文（带行号，支持行范围）
- `/write src/foo.rs` — 提取对话中最后一个代码块写入文件
- `/edit src/main.rs fn_old fn_new` — 精确字符串替换
- 所有写入操作自动备份，路径沙箱防穿越

### 项目感知

启动时自动扫描当前目录（最多 100 个文件，自动排除 node_modules/target/.git 等），注入 system prompt。AI 开箱就知道你的项目结构，不需要手动描述。

### 技能系统

输入 `/命令名` 执行预配置的 shell 命令，输出自动发送给 MiMo 分析。支持参数传递、5 个内置默认技能、TUI 内管理技能。

### 流式 UTF-8 安全解码

正确处理跨 chunk 的多字节字符分片问题，中文、emoji 不会断裂。多数同类实现直接 `String::from_utf8` 遇到分片就报错。

### 单二进制零依赖

```
cargo build --release
```

一个可执行文件，无需 Python/Node 运行时，无需安装外部服务。SSH 到远程服务器直接用。

### 多会话管理

会话自动保存，重启恢复上下文。Ctrl+N 新建会话，F2 切换会话，每 5 条消息自动持久化。

### 成本追踪

状态栏实时显示：
- 累计 token 数（K）
- 输入/输出分别计数（↓↑）
- 缓存命中率（♻）
- 费用（¥）— 基于 MiMo Token Plan 官方费率

## 快速开始

### 1. 获取 API Key

前往 [小米 MiMo Token Plan 平台](https://platform.xiaomimimo.com/#/console/subscription) 获取密钥。

### 2. 构建

```bash
git clone https://github.com/你的用户名/mimo-opt.git
cd mimo-opt
cargo build --release
```

### 3. 首次运行

```bash
./target/release/mimo-opt
```

首次运行会自动创建配置文件并提示填入 API Key：

```
MiMo-OPT 首次运行，请配置 API Key

配置文件: ~/.config/mimo-opt/config.json

请编辑配置文件，填入你的 MiMo Token Plan API Key：
  "api_key": "tp-你的密钥"
```

编辑配置文件后重新运行即可。

### 配置

配置文件位于 `~/.config/mimo-opt/config.json`：

**MiMo Token Plan（Anthropic 格式）**：
```json
{
  "api_key": "tp-你的密钥",
  "base_url": "https://token-plan-sgp.xiaomimimo.com/anthropic",
  "model": "mimo-v2-flash",
  "auth_type": "anthropic"
}
```

**MiMo API（OpenAI Bearer 格式）**：
```json
{
  "api_key": "你的密钥",
  "base_url": "https://api.xiaomimimo.com/v1",
  "model": "mimo-v2-flash",
  "auth_type": "bearer"
}
```

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `api_key` | (空) | API Key |
| `base_url` | `https://token-plan-sgp.xiaomimimo.com/anthropic` | API 端点 |
| `model` | `mimo-v2-flash` | 模型名称 |
| `auth_type` | `anthropic` | 认证方式：`anthropic`（api-key header）或 `bearer`（Authorization: Bearer） |
| `api_format` | `anthropic` | 数据格式：`anthropic`（Anthropic Messages）或 `openai`（OpenAI 兼容） |
| `max_tokens` | `4096` | 最大输出 token 数 |
| `skills` | `{}` | 技能命令映射，如 `{"lint": "cargo clippy 2>&1"}` |

`auth_type` 和 `api_format` 正交组合，适配不同 API 提供商：

| API | auth_type | api_format | base_url |
|-----|-----------|------------|----------|
| MiMo Token Plan | `anthropic` | `anthropic` | `https://token-plan-sgp.xiaomimimo.com/anthropic` |
| MiMo API | `bearer` | `openai` | `https://api.xiaomimimo.com/v1` |
| DeepSeek | `bearer` | `openai` | `https://api.deepseek.com` |
| GLM (智谱) | `bearer` | `openai` | `https://open.bigmodel.cn/api/paas/v4` |

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Enter` | 发送消息 / 执行技能 |
| `Ctrl+C` / `Esc` | 中断生成（不退出） |
| `Ctrl+Q` | 退出（有对话时弹确认框） |
| `Ctrl+N` | 新建会话 |
| `F2` | 切换会话 |
| `Ctrl+F` | 搜索消息 |
| `Ctrl+Y` | 复制最后一个代码块到剪贴板 |
| `↑/↓` | 浏览输入历史 |
| `PageUp/PageDown` | 翻页浏览对话 |
| `Ctrl+Home` | 回到对话底部 |
| `Home/End` | 光标行首/行尾 |
| `Ctrl+A/Ctrl+E` | 光标行首/行尾（Emacs 风格） |

## 内置命令

| 命令 | 功能 |
|------|------|
| `/read <path>[:range]` | 读取文件注入上下文（如 `/read src/main.rs:10-50`） |
| `/write <path>` | 提取对话中最后一个代码块写入文件 |
| `/edit <path> <old> <new>` | 精确字符串替换 |
| `/clear` | 清空当前对话 |
| `/skills` | 列出全部技能 |
| `/addskill <name> <cmd>` | 添加技能 |
| `/rmskill <name>` | 删除技能 |
| `/help` | 显示帮助 |
| `/skill_name [args]` | 执行技能命令（如 `/lint --fix`） |

## 技能系统

输入 `/命令名` 执行预配置的 shell 命令，输出自动显示在对话区并发送给 MiMo 分析。

配置示例（config.json 的 `skills` 字段）：
```json
"skills": {
  "lint": "cargo clippy 2>&1",
  "test": "cargo test 2>&1",
  "git": "git log --oneline -10",
  "diff": "git diff",
  "build": "cargo build 2>&1"
}
```

使用方式：
- 输入 `/lint` → 执行 `cargo clippy 2>&1` → MiMo 分析结果
- 输入 `/lint --fix` → 执行 `cargo clippy --fix 2>&1`（参数传递）
- 不配置 skills 时自动提供 5 个内置默认：lint/test/build/git/diff
- `/help` 列出全部可用技能
- `/skills` 查看详细列表
- `/addskill` 和 `/rmskill` 在 TUI 内管理技能

跨平台兼容：Windows 使用 `cmd /C`，其他平台使用 `sh -c`。

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust 2021 |
| TUI | ratatui + crossterm |
| 异步 | tokio |
| HTTP | reqwest（流式 + JSON） |
| 序列化 | serde + serde_json |
| 语法高亮 | syntect |
| 剪贴板 | cli-clipboard |
| 终端安全 | scopeguard |
| Unicode | unicode-width |

## 项目结构

```
src/
├── main.rs          # 入口，配置加载 + API Key 检查
├── config.rs        # 配置文件管理（JSON 读写 + save()）
├── app.rs           # 应用状态 + 事件循环 + 缓存策略 + 技能/文件/会话/确认（~1500 行）
├── file_ops.rs      # 文件操作（read/write/edit + 路径沙箱 + 自动备份）
├── session.rs       # 会话持久化（Session 结构 + JSON 存储）
├── api/
│   ├── mod.rs       # MiMoClient（流式请求 + SSE 解析）
│   └── types.rs     # API 类型定义（Content enum + 请求/响应/缓存）
└── ui/
    ├── mod.rs       # UI 模块导出
    ├── theme.rs     # Tokyo Night 配色
    └── draw.rs      # 界面渲染（标题栏/对话区/输入框/状态栏/确认弹窗/搜索栏）
```

## 与其他工具的对比

| 特性 | MiMo-OPT | 通用 API 终端工具 | 浏览器聊天 |
|------|----------|-------------------|------------|
| 缓存命中优化 | 前缀缓存 + 多断点策略 | 无 | 取决于平台 |
| 缓存命中率可视化 | ✅ 状态栏实时显示 | ❌ | ❌ |
| 代码块语法高亮 | ✅ syntect 100+ 语言 | 部分 | ✅ |
| 项目文件感知 | ✅ 自动扫描注入 | ❌ | ❌ |
| 文件操作 | ✅ read/write/edit + 备份 | ❌ | ❌ |
| 技能系统 | ✅ /命令 → MiMo 分析 | ❌ | ❌ |
| 多会话管理 | ✅ 自动保存 + 切换 | 部分 | ✅ |
| 流式 UTF-8 安全 | ✅ 跨 chunk 处理 | 部分 | ✅ |
| 单二进制部署 | ✅ ~5MB | 依赖运行时 | 不适用 |
| MiMo API 适配 | ✅ Token Plan + 普通 API | 需手动配置 | 需平台支持 |
| 成本追踪 | ✅ token + 缓存 + 费用 | 通常无 | 部分平台 |
| 确认机制 | ✅ 破坏性操作需确认 | 无 | 无 |
| 代码块复制 | ✅ Ctrl+Y 一键复制 | 无 | 手动 |
