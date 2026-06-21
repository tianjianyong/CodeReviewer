# CodeReviewer

AI 代码自动审核工具。专门检测 AI 生成代码中的常见问题：过度设计、结构臃肿、回退掩盖问题、测试简单、文档缺失等。

## 安装

```sh
pwsh -Command "cargo build --release"
```

## CLI 使用

```sh
# 扫描文件或目录
pwsh -Command "./target/release/codereviewer check src/"

# JSON 输出（给脚本或 AI 用）
pwsh -Command "./target/release/codereviewer check src/ --format json"

# 只运行指定规则
pwsh -Command "./target/release/codereviewer check src/ --rules R01,R02"

# 只显示 error 级别
pwsh -Command "./target/release/codereviewer check src/ --severity error"

# 列出所有规则
pwsh -Command "./target/release/codereviewer list-rules"
```

## MCP Server

MCP server 通过 stdio 传输，可被 opencode 等 MCP 客户端调用。

### opencode 配置

在项目根 `opencode.json` 中添加：

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "codereviewer": {
      "type": "local",
      "command": ["./target/release/codereviewer-mcp.exe"],
      "enabled": true
    }
  }
}
```

### 开发期间更新 MCP server

opencode 长持有 MCP 进程（stdio 协议要求），导致 `cargo build` 无法覆盖 exe。用这个脚本重建：

```sh
pwsh -Command "pwsh -File scripts/rebuild-mcp.ps1"
```

脚本会 kill 旧进程并编译 release 版本。之后**重启 opencode** 加载新版。

### 暴露的工具

- `review(path: string)` — 扫描路径，返回 JSON finding 列表
- `list_rules()` — 列出所有可用规则

## 检测规则（MVP 10 条）

| ID | 规则名 | 严重级 | 说明 |
|----|--------|--------|------|
| R01 | fallback-masks-error | error | catch/unwrap_or 吞错误 |
| R02 | structural-bloat | warning | 函数过长/嵌套过深/参数过多 |
| R03 | missing-doc | warning | public item 无 doc comment |
| R04 | simple-unit-test | warning | 测试断言稀薄 |
| R05 | shallow-integration-test | warning | 集成测试只验证状态码 |
| R06 | over-engineering | info | 单实现 trait/过度泛型 |
| R07 | dead-code | warning | 未用 import |
| R08 | todo-fixme-accumulation | info | TODO/FIXME 堆积 |
| R09 | commented-out-code | warning | 注释掉的代码块 |
| R10 | magic-number | info | 魔法数字/字符串 |

## 配置

在项目根创建 `.codereviewer.toml`：

```toml
[global]
exclude = ["target/", "vendor/"]

[rules.R02]
enabled = true
[rules.R02.thresholds]
max_function_lines = 50
max_nesting_depth = 4
max_parameters = 5

[rules.R06]
enabled = false
```

## 支持语言

Rust、Python、TypeScript (含 TSX)、C#、Java

## 技术栈

- Rust + Cargo workspace
- tree-sitter 多语言解析
- rmcp (MCP Rust SDK)
- TOML 配置驱动
