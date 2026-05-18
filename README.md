# MiMo-OPT

小米 MiMo API 终端聊天工具 —— Rust 编写的极简 客户端，接上 API Key 就能用。

> **测试阶段** — 项目仍在积极开发中，功能和接口可能变动。遇到 Bug 或有优化建议，欢迎联系作者 QQ：**2391859666**

## 为什么选 MiMo-OPT

### 缓存优化 — 直接省钱

大多数终端聊天工具每次请求都发送完整上下文，API 端全部重新计算。MiMo-OPT 实现了 Anthropic Messages API 的前缀缓存机制：

- **System prompt 永久缓存**：MiMo 身份、输出格式、项目文件列表作为固定锚点，整个会话不变，首次请求后稳定命中 50-70%
- **对话前缀断点**：长对话中早期轮次自动标记 `cache_control`，API 直接从缓存读取，命中率可达 70-90%
- **状态栏实时显示** `♻ XX%` 缓存命中率，花了多少钱一目了然

### 项目感知

启动时自动扫描当前目录（最多 100 个文件，自动排除 node_modules/target/.git 等），注入 system prompt。AI 开箱就知道你的项目结构，不需要手动描述。

### 流式 UTF-8 安全解码

正确处理跨 chunk 的多字节字符分片问题，中文、emoji 不会断裂。多数同类实现直接 `String::from_utf8` 遇到分片就报错。

### 单二进制零依赖

```
cargo build --release
```

一个可执行文件，无需 Python/Node 运行时，无需安装外部服务。SSH 到远程服务器直接用。

### 成本追踪

状态栏实时显示：
- 累计 token 数（K）
- 输入/输出分别计数（↓↑）
- 费用（¥）— 基于 MiMo Token Plan 官方费率
- 缓存命中率（♻）

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
| `skills` | `{}` | 技能命令映射，如 `{"lint": "cargo clippy 2>&1"}` |

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Enter` | 发送消息 |
| `Ctrl+C` | 中断生成（不退出） |
| `Esc` | 中断生成 |
| `Ctrl+Q` | 退出 |
| `←/↑/→/↓` | 移动光标 |
| `Home / Ctrl+A` | 行首 |
| `End / Ctrl+E` | 行尾 |
| `Backspace / Delete` | 删除字符 |

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
- 输入 `/test` → 执行 `cargo test 2>&1` → MiMo 分析结果
- 未知命令显示 "未知技能" 错误

跨平台兼容：Windows 使用 `cmd /C`，其他平台使用 `sh -c`。

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust 2021 |
| TUI | ratatui + crossterm |
| 异步 | tokio |
| HTTP | reqwest（流式 + JSON） |
| 序列化 | serde + serde_json |
| 终端安全 | scopeguard |
| Unicode | unicode-width |

## 项目结构

```
src/
├── main.rs          # 入口，配置加载 + API Key 检查
├── config.rs        # 配置文件管理（JSON 读写）
├── app.rs           # 应用状态 + 事件循环 + 缓存策略
├── api/
│   ├── mod.rs       # MiMoClient（流式请求 + SSE 解析）
│   └── types.rs     # API 类型定义（请求/响应/缓存）
└── ui/
    ├── mod.rs       # UI 模块导出
    ├── theme.rs     # Tokyo Night 配色
    └── draw.rs      # 界面渲染（标题栏/对话区/输入框/状态栏）
```

## 与其他工具的对比

| 特性 | MiMo-OPT | 通用 API 终端工具 | 浏览器聊天 |
|------|----------|-------------------|------------|
| 缓存命中优化 | 前缀缓存 + 断点策略 | 无 | 取决于平台 |
| 缓存命中率可视化 | ✅ 状态栏实时显示 | ❌ | ❌ |
| 项目文件感知 | ✅ 自动扫描注入 | ❌ | ❌ |
| 技能系统 | ✅ /命令 → MiMo 分析 | ❌ | ❌ |
| 流式 UTF-8 安全 | ✅ 跨 chunk 处理 | 部分 | ✅ |
| 单二进制部署 | ✅ ~5MB | 依赖运行时 | 不适用 |
| MiMo API 适配 | ✅ Token Plan + 普通 API | 需手动配置 | 需平台支持 |
| 成本追踪 | ✅ token + 费用 | 通常无 | 部分平台 |

## License

MIT
