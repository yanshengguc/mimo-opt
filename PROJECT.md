# MiMo-OPT — 小米 MiMo API 终端聊天工具

> 极简 TUI，接上 MiMo Token Plan API 就能用

---

## 当前目标：最小可用版本

直接能跑，接上 API Key 就能聊天。

### 功能范围

- [ ] MiMo Token Plan API 流式对话
- [ ] ratatui 终端界面（对话区 + 输入框 + 状态栏）
- [ ] 配置文件存 API Key
- [ ] 流式逐 token 输出
- [ ] Ctrl+C 退出

### 技术栈

- Rust + ratatui + crossterm + tokio + reqwest + serde
- API：`https://token-plan-sgp.xiaomimimo.com/anthropic`（Anthropic 兼容）

### 以后再做

- 多会话、工具系统、Agent、缓存优化、Tauri GUI...
