# CodeReviewer 设计文档

> 配套：[需求文档](../requirements/requirements.md) | [实施跟踪](../tracking/implementation-tracker.md)

## 1. 架构总览

```
┌──────────────────────────────────────────────────────────┐
│                      用户接入层                          │
│   CLI (codereviewer)            MCP Server (stdio)       │
│      ↓                              ↓                    │
│      └──────────┬───────────────────┘                   │
│                 ↓                                         │
├──────────────────────────────────────────────────────────┤
│                      core 引擎                           │
│   ┌─────────────┐  ┌──────────────┐  ┌──────────────┐   │
│   │  Config     │  │  Analyzer    │  │  Reporter    │   │
│   │  (TOML 加载)│  │  (调度规则)  │  │  (输出格式)  │   │
│   └─────────────┘  └──────┬───────┘  └──────────────┘   │
│                           ↓                              │
│   ┌─────────────────────────────────────────────────┐   │
│   │  Rule trait + 内置规则集（10 条）                │   │
│   └─────────────────────────────────────────────────┘   │
│                           ↓                              │
│   ┌─────────────────────────────────────────────────┐   │
│   │  Parser 层（tree-sitter 多语言）                 │   │
│   └─────────────────────────────────────────────────┘   │
│                           ↓                              │
│   ┌─────────────────────────────────────────────────┐   │
│   │  LlmReviewer trait（Phase 2 空壳，MVP 不实现）   │   │
│   └─────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

## 2. Cargo Workspace 结构

```
CodeReviewer/
├─ Cargo.toml                 # workspace 根
├─ crates/
│  ├─ core/                   # 引擎与 Rule trait
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ config.rs         # TOML 配置加载
│  │     ├─ finding.rs        # Finding / Severity 数据结构
│  │     ├─ rule.rs           # Rule trait
│  │     ├─ analyzer.rs       # 规则调度引擎
│  │     ├─ parser.rs         # tree-sitter 封装
│  │     ├─ reporter.rs       # 终端彩色 + JSON 输出
│  │     └─ llm.rs            # LlmReviewer trait 空壳
│  ├─ rules/                  # 内置规则集
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs            # 注册全部规则
│  │     ├─ r01_fallback.rs
│  │     ├─ r02_bloat.rs
│  │     ├─ r03_missing_doc.rs
│  │     ├─ r04_simple_unit_test.rs
│  │     ├─ r05_shallow_integration.rs
│  │     ├─ r06_over_engineering.rs
│  │     ├─ r07_dead_code.rs
│  │     ├─ r08_todo.rs
│  │     ├─ r09_commented_code.rs
│  │     └─ r10_magic_number.rs
│  ├─ cli/                    # 二进制入口
│  │  ├─ Cargo.toml
│  │  └─ src/main.rs
│  └─ mcp/                    # MCP server
│     ├─ Cargo.toml
│     └─ src/lib.rs
└─ docs/
```

**依赖方向**：`cli` 与 `mcp` 都依赖 `core` 与 `rules`；`rules` 依赖 `core`；`core` 不依赖任何兄弟 crate。无环依赖。

## 3. 核心数据结构

### 3.1 Finding 与 Severity

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity { Error, Warning, Info }

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,      // 1-indexed
    pub column: usize,    // 1-indexed
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: String,       // "R01"
    pub rule_name: String,     // "fallback-masks-error"
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub snippet: Option<String>, // 可选：上下文代码片段
}
```

### 3.2 Rule trait

```rust
pub trait Rule {
    fn id(&self) -> &'static str;          // "R01"
    fn name(&self) -> &'static str;        // "fallback-masks-error"
    fn severity(&self) -> Severity;
    fn languages(&self) -> &'static [Language];

    /// 在已解析的文件上跑分析，返回 finding 列表
    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError>;
}
```

### 3.3 AnalysisContext

```rust
pub struct AnalysisContext<'a> {
    pub source: &'a str,
    pub tree: &'a tree_sitter::Tree,
    pub language: Language,
    pub config: &'a RuleConfig,    // 当前规则的配置项
    pub file_path: &'a Path,
}
```

规则通过 `ctx.tree.root_node()` 拿到 AST，用 query 或遍历节点方式分析。

### 3.4 LlmReviewer trait（空壳，MVP 不实现）

```rust
pub trait LlmReviewer {
    /// 对规则产生的疑似 finding 进行二次确认
    fn review(&self, findings: &[Finding]) -> Result<Vec<ReviewVerdict>, LlmError>;
}

pub struct ReviewVerdict {
    pub original: Finding,
    pub confirmed: bool,
    pub comment: String,
}
```

MVP 仅定义 trait 与 `ReviewVerdict`，无实现。Phase 2 接入 OpenAI/Anthropic/ollama。

## 4. 检测引擎流程

```
1. 加载配置（--config > 项目根 .codereviewer.toml > 内置默认）
2. 枚举目标路径下文件（按语言识别扩展名，应用 exclude glob）
3. 对每个文件：
   a. 读取源码
   b. tree-sitter 解析得 Tree
   c. 取语言对应的规则子集（按 rule.languages() 过滤）
   d. 顺序跑每条规则（单条规则 panic 用 catch_unwind 隔离）
   e. 收集 Finding
4. 按 severity 排序，调用 Reporter 输出
```

**规则调度**：MVP 顺序执行，不做并行。性能瓶颈预期在解析而非规则，且顺序更易调试。Phase 2.5 可加并行。

## 5. 规则系统设计

### 5.1 内置规则

每条规则是 `rules/` crate 中的一个 struct，实现 `Rule` trait。规则在 `rules/src/lib.rs` 中统一注册：

```rust
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FallbackMasksError::default()),
        Box::new(StructuralBloat::default()),
        // ...
    ]
}
```

### 5.2 TOML 配置格式

```toml
# .codereviewer.toml

[global]
exclude = ["target/", "vendor/", "*.generated.rs"]

[rules.R01]                    # 启用并调阈值
enabled = true
severity = "error"
[rules.R01.thresholds]
unwrap_or_count = 3

[rules.R02]
enabled = true
[rules.R02.thresholds]
max_function_lines = 50
max_nesting_depth = 4
max_parameters = 5

[rules.R06]
enabled = false                # 关闭某条规则
```

### 5.3 配置加载顺序

1. 内置 `Default::default()` 的 `RuleConfig`
2. 项目根 `.codereviewer.toml`（若存在）
3. `--config <path>` 指定的文件（最高优先级）

逐层覆盖，深层覆盖浅层。

## 6. tree-sitter 集成

### 6.1 语言映射

```rust
pub enum Language {
    Rust,
    Python,
    TypeScript,
    CSharp,
    Java,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "ts" | "tsx" => Some(Language::TypeScript),
            "cs" => Some(Language::CSharp),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    pub fn tree_sitter_language(&self) -> tree_sitter::Language { /* ... */ }
}
```

### 6.2 依赖

- `tree-sitter` crate
- `tree-sitter-rust`、`tree-sitter-python`、`tree-sitter-typescript`、`tree-sitter-c-sharp`、`tree-sitter-java` grammar

### 6.3 解析失败处理

- 解析返回错误时，跳过该文件，在 reporter 中记录一条 `info` 级提示
- 不中断整体扫描

## 7. CLI 输出格式

### 7.1 终端彩色（默认）

```
R01 error  src/api.rs:42:9   fallback masks error: unwrap_or() hides None case
     │
  42 │     let x = opt.unwrap_or(default);
     │                       ^^^^^^^

R02 warning src/api.rs:120:5  function too long: 78 lines (max 50)
```

- error: 红色，warning: 黄色，info: 蓝色
- 末尾汇总：`Found 12 findings (3 errors, 7 warnings, 2 infos) in 18 files`

### 7.2 JSON 输出（`--format json`）

```json
{
  "summary": { "errors": 3, "warnings": 7, "infos": 2, "files": 18 },
  "findings": [
    {
      "rule_id": "R01",
      "rule_name": "fallback-masks-error",
      "severity": "error",
      "location": { "file": "src/api.rs", "line": 42, "column": 9 },
      "message": "unwrap_or() hides None case",
      "snippet": null
    }
  ]
}
```

## 8. MCP Server 设计

### 8.1 传输

stdio，遵循 MCP 协议规范。

### 8.2 暴露工具

**`review`**
- 输入：`{ "path": "<目录或文件>", "format": "json" }`
- 输出：结构化 finding 列表（同 CLI JSON 格式）

**`list_rules`**
- 输入：`{}`
- 输出：`[{ "id": "R01", "name": "...", "severity": "error", "languages": ["rust"] }, ...]`

### 8.3 实现

使用 `rmcp` crate（Rust MCP SDK）。`mcp` crate 把 `core::analyze_path` 包装成 MCP tool handler。

## 9. 关键依赖选型

| 用途 | crate | 备注 |
|-----|-------|------|
| 解析 | `tree-sitter` + 各语言 grammar | 多语言 AST |
| 配置 | `toml` + `serde` | TOML 配置 |
| CLI | `clap` | 命令行解析 |
| 彩色输出 | `anstream` + `anstyle` | 跨平台终端颜色 |
| JSON | `serde_json` | 输出与配置反序列化 |
| 错误 | `thiserror` | 库错误；CLI 用 `anyhow` |
| MCP | `rmcp` | 官方 Rust MCP SDK |
| 日志 | `tracing` | 结构化日志（Phase 4 加） |

## 10. 错误处理策略

- `core` 与 `rules`：用 `thiserror` 定义具体错误类型，返回 `Result`
- `cli` 与 `mcp`：用 `anyhow` 聚合，顶层打印友好信息
- 规则 panic：`analyzer` 用 `std::panic::catch_unwind` 隔离单条规则，记为 `RuleError::Panic`
- 文件 IO 错误：跳过文件并记日志，不中断

## 11. 测试策略

- **单元测试**：每条规则在 `rules/` crate 内附 `#[cfg(test)] mod tests`，使用 inline 源码片段
- **fixtures**：`crates/core/tests/fixtures/` 放置问题代码样本
- **集成测试**：`crates/core/tests/integration.rs` 跑完整 `analyze_path`，断言 finding 数量与类型
- **快照测试**：用 `insta` crate 锁定 JSON 输出格式（防止意外破坏输出契约）

## 12. 关键设计决策记录

| 决策 | 选择 | 替代方案 | 理由 |
|-----|------|---------|------|
| MVP 范围 | CLI + MCP | 含 LLM / 仅 CLI | MCP 早做可自用狗粮，LLM 推 phase 2 |
| 解析方案 | tree-sitter | 仅 Rust / 自写 parser | 一开始多语言，避免后期重构 |
| 规则系统 | TOML 配置 | 硬编码 / WASM 插件 | 可调不需改代码，复杂度适中 |
| MCP 传输 | stdio | HTTP/SSE | 最简单，opencode 原生支持 |
| LLM | trait 空壳 | 不做 / 立刻做 | 留接口避免后期大改 |
| 严重级 | 3 级 | 5 级 | 3 级够用，避免过度设计 |
| 输出格式 | 终端 + JSON | + SARIF | SARIF 推 phase 3 |
| 配置文件 | 项目根 .codereviewer.toml | 多层嵌套 | Simplicity First |
| 规则调度 | 顺序 | 并行 | 性能瓶颈在解析，顺序更易调试 |
| 首批语言 | Rust + Python + TypeScript + C# + Java | 单 Rust | 多语言验证 tree-sitter 架构 |
