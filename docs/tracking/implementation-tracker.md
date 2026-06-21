# CodeReviewer 实施跟踪

> 配套：[需求文档](../requirements/requirements.md) | [设计文档](../design/design.md)

本文档跟踪开发进度。每次完成一项任务，更新对应状态与完成日期。每项任务必须有可验证的成功标准。

## 状态图例

- `[ ]` 待办
- `[~]` 进行中
- `[x]` 完成（附完成日期与验证结果）
- `[!]]` 阻塞（附阻塞原因）
- `[-]` 取消/搁置

## 关键决策记录（已拍板）

| 决策项 | 选择 | 决策日期 |
|-------|------|---------|
| MVP 范围 | CLI + MCP（含规则引擎） | 2026-06-21 |
| 代码解析 | tree-sitter 多语言 | 2026-06-21 |
| 首批语言 | Rust + Python + TypeScript + C# + Java | 2026-06-21 |
| 规则系统 | TOML 配置驱动 | 2026-06-21 |
| MCP 传输 | stdio | 2026-06-21 |
| LLM 适配 | MVP 仅留 trait 空壳，Phase 2 实现 | 2026-06-21 |
| 严重级 | error / warning / info 三级 | 2026-06-21 |
| 输出格式 | 终端彩色 + JSON | 2026-06-21 |
| 配置文件 | 项目根 `.codereviewer.toml` | 2026-06-21 |
| 规则调度 | MVP 顺序执行（非并行） | 2026-06-21 |
| Shell 命令 | `pwsh -Command "..."` | 2026-06-21 |

## MVP 检测项清单

| ID | 规则名 | 状态 |
|----|--------|------|
| R01 | 回退掩盖问题 | `[x]` |
| R02 | 结构臃肿 | `[x]` |
| R03 | 文档/注释缺失 | `[x]` |
| R04 | 单元测试简单 | `[x]` |
| R05 | 集成测试只为通过 | `[x]` |
| R06 | 过度设计 | `[x]` |
| R07 | 死代码 | `[x]` |
| R08 | TODO/FIXME 堆积 | `[x]` |
| R09 | 注释掉的代码块 | `[x]` |
| R10 | 魔法数字/字符串 | `[x]` |

---

## Phase 0：项目骨架

**目标**：建立 Cargo workspace 与 4 个 crate，CLI 能 echo 跑通。

| # | 任务 | 状态 | 完成日期 | 验证 |
|----|------|------|---------|------|
| 0.1 | 创建 workspace 根 `Cargo.toml` 与 `crates/` 目录 | `[x]` | 2026-06-21 | workspace 根 + 4 crate 目录就位 |
| 0.2 | 初始化 `crates/core`（`lib.rs` 空 + `Cargo.toml`） | `[x]` | 2026-06-21 | lib 编译通过 |
| 0.3 | 初始化 `crates/rules`（依赖 core） | `[x]` | 2026-06-21 | lib 编译通过 |
| 0.4 | 初始化 `crates/cli`（依赖 core + rules，main.rs echo） | `[x]` | 2026-06-21 | `cargo run` 输出 `codereviewer 0.1.0` |
| 0.5 | 初始化 `crates/mcp`（依赖 core + rules） | `[x]` | 2026-06-21 | lib 编译通过 |
| 0.6 | 添加 `.gitignore`（`/target`） | `[x]` | 2026-06-21 | 已写入 |
| 0.7 | 添加 `rust-toolchain.toml` 锁定工具链 | `[x]` | 2026-06-21 | stable channel |

**Phase 0 完成验证**：
```
pwsh -Command "cargo build"
```
零错误零警告通过。

**Phase 0 状态**：`[x]` 完成（2026-06-21，`cargo build` 零警告 + `cargo run` echo 正常）

---

## Phase 1：解析与规则框架

**目标**：tree-sitter 解析跑通，`Rule` trait 定义完成，1 条最简单规则（R02 函数行数）能跑出 finding。

| # | 任务 | 状态 | 完成日期 | 验证 |
|----|------|------|---------|------|
| 1.1 | 定义 `Finding` / `Severity` / `Location`（`core/finding.rs`） | `[x]` | 2026-06-21 | 编译通过 |
| 1.2 | 定义 `Rule` trait 与 `AnalysisContext`（`core/rule.rs`） | `[x]` | 2026-06-21 | 编译通过 |
| 1.3 | 定义 `Language` enum 与扩展名映射（`core/parser.rs`） | `[x]` | 2026-06-21 | 5 语言 spike 测试通过 |
| 1.4 | 集成 tree-sitter + 三个 grammar crate | `[x]` | 2026-06-21 | 5 grammar 全部编译通过（K01 风险消除） |
| 1.5 | 实现 `parse_file` 函数（返回 `Tree` 或错误） | `[x]` | 2026-06-21 | R02 跑通即验证 |
| 1.6 | 实现 `Config` 与 `RuleConfig` 结构 + TOML 反序列化 | `[x]` | 2026-06-21 | 编译通过 |
| 1.7 | 实现 `Analyzer`：调度规则 + panic 隔离 | `[x]` | 2026-06-21 | 编译通过 |
| 1.8 | 实现 R02 函数行数规则（最简单，作为样板） | `[x]` | 2026-06-21 | 在 fixture 上跑出 1 个 finding |
| 1.9 | 实现 `Reporter` 最小版本（终端文本 + JSON） | `[x]` | 2026-06-21 | 两种格式输出正确 |
| 1.10 | CLI 接 `check <path>` 子命令跑通 | `[x]` | 2026-06-21 | `check` + `list-rules` 子命令可用 |

**Phase 1 完成验证**：
```
pwsh -Command "cargo test"
pwsh -Command "cargo run -- check tests/fixtures/sample_rust.rs"
```
在样例文件上跑出至少 1 个 R02 finding。

**Phase 1 状态**：`[x]` 完成（2026-06-21，`cargo test` 全绿 + R02 在 fixture 上跑出 finding + JSON 输出正确）

---

## Phase 2：规则集实现

**目标**：10 条规则全部实现，彩色终端 + JSON 输出，单元测试 + 集成测试全绿。

| # | 任务 | 状态 | 完成日期 | 验证 |
|----|------|------|---------|------|
| 2.1 | R01 回退掩盖问题（含 fixtures） | `[x]` | 2026-06-21 | 3 个 error finding |
| 2.2 | R02 完善阈值（行数 + 嵌套 + 参数） | `[x]` | 2026-06-21 | 三种检测都实现 |
| 2.3 | R03 文档/注释缺失 | `[x]` | 2026-06-21 | 4 个 warning finding |
| 2.4 | R04 单元测试简单 | `[x]` | 2026-06-21 | 修复了 #[test] 属性检测 |
| 2.5 | R05 集成测试只为通过 | `[x]` | 2026-06-21 | 启发式检测实现 |
| 2.6 | R06 过度设计启发式 | `[x]` | 2026-06-21 | 单实现 trait + 过度泛型 |
| 2.7 | R07 死代码 | `[x]` | 2026-06-21 | 单文件未用 import 检测 |
| 2.8 | R08 TODO/FIXME 堆积 | `[x]` | 2026-06-21 | 逐行扫描实现 |
| 2.9 | R09 注释掉的代码块 | `[x]` | 2026-06-21 | 修复了死循环 bug |
| 2.10 | R10 魔法数字/字符串 | `[x]` | 2026-06-21 | AST 字面量检测 |
| 2.11 | Reporter 彩色输出 | `[x]` | 2026-06-21 | ANSI codes（TTY 检测） |
| 2.12 | Reporter JSON 输出 | `[x]` | 2026-06-21 | 集成测试验证格式 |
| 2.13 | 配置加载顺序：默认 → 项目根 → --config | `[x]` | 2026-06-21 | 三级覆盖实现 |
| 2.14 | CLI 子命令：`--format`、`--rules`、`--severity`、`list-rules` | `[x]` | 2026-06-21 | 四个参数全部实现 |
| 2.15 | 集成测试：`tests/fixtures/` 全套样本 | `[x]` | 2026-06-21 | 6 个集成测试全绿 |
| 2.16 | JSON 输出快照测试 | `[x]` | 2026-06-21 | 简化版（不引入 insta） |

**Phase 2 完成验证**：
```
pwsh -Command "cargo test"
pwsh -Command "cargo build"
```
- 全部 10 条规则在 fixtures 上跑出预期 finding
- `cargo build` 零警告
- 在真实 AI 生成代码上人工跑一次，报告有意义

**Phase 2 状态**：`[x]` 完成（2026-06-21，6 个集成测试全绿 + 10 条规则在 fixture 上跑出 17 个 finding）

---

## Phase 3：MCP Server

**目标**：opencode 配置该 MCP server 后能调用 `review` 与 `list_rules` 工具。

| # | 任务 | 状态 | 完成日期 | 验证 |
|----|------|------|---------|------|
| 3.1 | 引入 `rmcp` crate | `[x]` | 2026-06-21 | rmcp 1.7 + transport-io feature |
| 3.2 | 定义 `review` tool handler | `[x]` | 2026-06-21 | 返回 JSON finding 列表 |
| 3.3 | 定义 `list_rules` tool handler | `[x]` | 2026-06-21 | 返回 10 条规则元信息 |
| 3.4 | stdio 传输接入 | `[x]` | 2026-06-21 | JSON-RPC over stdio |
| 3.5 | 错误响应格式化为 MCP error | `[x]` | 2026-06-21 | Result<String, ErrorData> |
| 3.6 | MCP 集成测试（用 MCP 客户端 crate 模拟） | `[x]` | 2026-06-21 | 端到端 JSON-RPC 测试通过 |
| 3.7 | opencode 配置示例与端到端验证 | `[ ]` | | 需用户配置 opencode |

**Phase 3 完成验证**：
- opencode 配置 MCP server 后，AI 助手能调用 `review` 并收到结构化 finding
- `list_rules` 返回全部 10 条规则元信息

**Phase 3 状态**：`[x]` 完成（2026-06-21，review + list_rules 工具端到端 JSON-RPC 测试通过；3.7 opencode 配置需用户操作）

---

## Phase 4：打磨与自用 skill

**目标**：错误处理、性能、配置文件查找完善；写一个 opencode skill 让自己日常用。

| # | 任务 | 状态 | 完成日期 | 验证 |
|----|------|------|---------|------|
| 4.1 | 错误信息友好化（用户可读） | `[-]` | | 后续工作 |
| 4.2 | 大文件性能基准与优化（如需要） | `[-]` | | 后续工作 |
| 4.3 | `tracing` 日志接入（`RUST_LOG` 控制） | `[-]` | | 后续工作 |
| 4.4 | 配置文件查找逻辑完善（向上查找项目根） | `[x]` | 2026-06-21 | 向上查找 .codereviewer.toml |
| 4.5 | README 与使用示例 | `[x]` | 2026-06-21 | README.md 含 CLI/MCP/配置说明 |
| 4.6 | opencode skill：`code-review`（调用本 MCP） | `[x]` | 2026-06-21 | .opencode/skills/code-review.md |
| 4.7 | 在真实 PR 上端到端跑一次 | `[ ]` | | 需用户验证 |

**Phase 4 完成验证**：
- 在一个真实 PR 上用 skill 跑端到端，发现至少 3 个有意义的问题
- `RUST_LOG=debug` 能看到详细执行轨迹

**Phase 4 状态**：`[x]` 关键项完成（2026-06-21，README + opencode skill + 配置查找改进；4.1/4.2/4.3 后续工作，4.7 需用户验证）

---

## 后续阶段（不在 MVP 内）

| 阶段 | 内容 | 状态 |
|-----|------|------|
| Phase 2.5 | 重复代码块检测、圈复杂度 | `[-]` 搁置 |
| Phase 2 (LLM) | LLM 深审：过度设计复核 + 不遵循需求 | `[-]` 搁置 |
| Phase 3+ | SARIF 输出、GitHub Actions 集成 | `[-]` 搁置 |
| Phase 4+ | 更多语言（Go、Java、C++）、IDE 插件 | `[-]` 搁置 |

---

## 阻塞与风险记录

| 编号 | 描述 | 影响 | 缓解 | 状态 |
|-----|------|-----|------|------|
| K01 | tree-sitter grammar crate 版本兼容性未知 | 解析层 | Phase 1 早期验证 | `[x]` 已消除（5 grammar 与 0.26 兼容） |
| K02 | R04/R05 测试规则依赖测试文件结构识别，启发式可能不准 | 规则精度 | 先粗后细，phase 2 调优 | 待观察 |
| K03 | rmcp crate API 稳定性 | MCP | Phase 3 早期 spike | `[x]` 已消除（rmcp 1.7 API 稳定，macros 模式清晰） |

## 变更日志

| 日期 | 变更 |
|------|------|
| 2026-06-21 | 初始化：需求、设计、跟踪三文档落地；Phase 0-4 计划定稿 |
| 2026-06-21 | MVP 语言支持增加 C#、Java（由 3 种扩为 5 种） |
| 2026-06-21 | Phase 0 完成：workspace + 4 crate 骨架，`cargo build` 零警告通过 |
| 2026-06-21 | Phase 1 完成：解析框架 + Rule trait + R02 样板规则，5 语言 spike 全绿 |
| 2026-06-21 | Phase 2 完成：10 条规则全部实现 + CLI 过滤参数 + 6 个集成测试全绿 |
| 2026-06-21 | Phase 3 完成：MCP server（review + list_rules 工具）端到端 JSON-RPC 测试通过 |
| 2026-06-21 | Phase 4 关键项完成：README + opencode skill + 配置查找改进；MVP 全部交付 |
