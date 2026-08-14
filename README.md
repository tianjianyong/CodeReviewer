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

# Markdown 汇总报告（统计 + 最严重问题 + 修复建议 + 重灾区文件）
pwsh -Command "./target/release/codereviewer check src/ --format md"

# 报告写入文件（默认打印到终端）
pwsh -Command "./target/release/codereviewer check src/ --format md --output report.md"
pwsh -Command "./target/release/codereviewer check src/ --format json --output report.json"

# 与上次报告对比，只报新增问题（CI 增量门禁：只有新 error 才非零退出）
pwsh -Command "./target/release/codereviewer check src/ --baseline report.json"

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
      "command": ["cargo", "run", "--release", "-p", "codereviewer-mcp"],
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

## 检测规则（19 条）

### 基础规则（R01–R10）

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

### AI 代码专项规则（R14–R28）

针对 AI vibe coding 高频缺陷，详见 [候选规则清单](docs/design/rule-candidates.md)。

| ID | 规则名 | 严重级 | 说明 |
|----|--------|--------|------|
| R14 | hardcoded-secret | error | 硬编码 API key/token/私钥 |
| R15 | missing-input-validation | warning | 边界函数对外部输入无空值/越界校验 |
| R16 | self-validating-test | warning | 测试名只描述方法不描述行为/访问私有成员 |
| R18 | async-missing-await | error | async 函数调用未加 await |
| R19 | n-plus-one-query | warning | ORM 循环访问关系字段触发 N+1 查询 |
| R20 | resource-leak | warning | open/addEventListener 无配对 close/remove |
| R23 | wrong-error-type-propagation | warning | 宽泛 catch 返回固定值丢错误类型信息 |
| R24 | hardcoded-path-or-url | warning | 源码硬编码绝对路径/URL |
| R28 | overly-defensive-handling | info | 对不可失败操作套 try/catch/unwrap_or |

> 候选未实现规则（R11/R12/R13/R17/R21/R22/R25/R26/R27）见 `docs/design/rule-candidates.md`。

## 配置

### 内置默认排除

CodeReviewer 自动跳过常见非源码目录，无需手动配置：

`node_modules`、`target`、`obj`、`bin`、`dist`、`build`、`out`、`.git`、`.vscode`、`.idea`、`__pycache__`、`.pytest_cache`、`vendor`、`.next`、`.nuxt`、`coverage`、`.cache`

### .gitignore 支持

CodeReviewer 自动读取扫描目录下的 `.gitignore` 文件并排除其中声明的模式。支持：

- 目录排除（`dir/`）
- 后缀 glob（`*.log`、`*.tmp`）
- 文件/目录名（`dist`）
- 注释（`#`）和空行跳过

复杂语法（`!` 取反、`**` 递归 glob、`[Dd]` 字符类）暂不支持，但这些目录通常已被内置默认排除覆盖。

### 项目特定排除

配置文件支持两种位置（向上查找，优先前者）：

- `.codereviewer/config.toml`（推荐：与报告同目录，统一收在项目一个隐藏目录里）
- 根目录 `.codereviewer.toml`（旧约定，兼容）

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

### 规则级文件排除

`rules.<ID>.exclude_files` 按文件名子串（不区分大小写）让某条规则跳过特定文件：

```toml
[rules.R24]
exclude_files = ["NavisworksTestAutomationClient", "TestAutomationHttpService"]
```

## 支持语言

解析层：Rust、Python、TypeScript (含 TSX)、C#、Java。

部分规则只实现了语言子集：R18/R19/R20 仅 Python/TS/TSX，R23 不含 Rust，R28 仅 Rust/TS/TSX，R06 仅 Rust，R07 不含 C#（using 是命名空间，单文件名字匹配原理上不可行）。

## 在其他项目中使用

推荐的目录约定：配置与报告统一收在目标项目的 `.codereviewer/` 目录，不污染项目根目录。

```sh
# 首次使用：建目录 + 可选调优配置
mkdir .codereviewer

# 扫描并输出报告到 .codereviewer/
pwsh -Command "./target/release/codereviewer check src/ --format md --output .codereviewer/report.md"
pwsh -Command "./target/release/codereviewer check src/ --format json --output .codereviewer/report.json"

# 增量门禁：只报新增问题
pwsh -Command "./target/release/codereviewer check src/ --baseline .codereviewer/report.json"
```

建议在目标项目 `.gitignore` 中：提交 `config.toml`（团队共享调优），忽略 `report.*`（可再生产物）。

## 技术栈

- Rust + Cargo workspace
- tree-sitter 多语言解析
- rmcp (MCP Rust SDK)
- TOML 配置驱动

## 发版流程

1. 同时更新 `Cargo.toml`（workspace `version`）与 `VERSION.md`——两者必须一致，测试 `version_matches_version_md` 会拦截不一致
2. 把 `CHANGELOG.md` 的 `[Unreleased]` 转成带日期的版本节（`[x.y.z] - YYYY-MM-DD`）
3. `cargo fmt --check && cargo test --workspace` 全绿（CI 同样会跑这两步）
4. 打 tag：`git tag v<版本号>`（如 `git tag v0.1.0`）
5. `cargo build --release` 构建产物作为 release asset 发布，不要提交进仓库
