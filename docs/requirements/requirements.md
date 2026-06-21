# CodeReviewer 需求文档

## 1. 背景与动机

当前大量项目由 AI 代码助手生成代码。AI 生成的代码普遍存在以下问题：

- 过度设计（多余抽象层、未使用的配置项、单实现 trait）
- 结构臃肿（超长函数、过深嵌套、参数过多）
- 用回退掩盖问题（`catch` 后返回默认值、`unwrap_or` 吞错误）
- 单元测试简单（只覆盖 happy path、断言稀薄、mock 泛滥）
- 集成测试只为通过（只断言状态码、不验证副作用）
- 注释和文档缺失（public item 无 doc comment）
- 不严格遵循需求（实现偏离需求语义）
- 死代码、TODO 堆积、注释掉的代码块、魔法数字

多数程序员经验不足或时间不够，无法认真审核代码，导致大量低质量代码进入代码库。

**CodeReviewer 的目标**：采用代码审核最佳实践，自动化发现上述问题。

## 2. 目标用户

- **主要**：使用 AI 代码助手的开发者，需要在合并前自动审查 AI 生成代码
- **次要**：经验不足的程序员，作为代码审核学习工具
- **集成**：AI 代码助手自身（通过 MCP 调用，在生成代码后自审）

## 3. MVP 范围

### 3.1 包含

- Rust 实现的 CLI 工具 `codereviewer`
- 基于规则的静态分析引擎（tree-sitter 多语言解析）
- TOML 配置驱动的规则系统
- MCP server（stdio 传输），供 AI 助手调用
- 终端彩色输出 + JSON 输出
- 五种语言支持：Rust、Python、TypeScript、C#、Java

### 3.2 不包含（明确排除）

- LLM 深度审查（Phase 2，MVP 仅留 `LlmReviewer` trait 空壳）
- "不遵循需求"检测（必须 LLM + 需求文档，Phase 2）
- 重复代码块检测、圈复杂度（Phase 2.5 备选）
- SARIF 输出格式（Phase 5）
- HTTP/SSE MCP 传输（MVP 仅 stdio）
- IDE 插件、CI 集成（后续阶段）
- 动态/WASM 插件系统（不做，TOML 配置已足够）

## 4. 功能性需求

### 4.1 检测项（MVP 10 项）

| ID | 问题类型 | 检测思路 | 严重级默认 |
|----|---------|---------|-----------|
| R01 | 回退掩盖问题 | `catch + return default`、`unwrap_or(默认值)` 吞错误、`?` 后紧接 fallback | error |
| R02 | 结构臃肿 | 函数行数 / 文件行数 / 嵌套深度 / 参数个数阈值 | warning |
| R03 | 文档/注释缺失 | public item 无 doc comment | warning |
| R04 | 单元测试简单 | 断言密度、mock 占比、happy path 比例 | warning |
| R05 | 集成测试只为通过 | 只 assert 状态码、不验证副作用 | warning |
| R06 | 过度设计 | 启发式：单实现 trait、未用配置项、过度泛型化 | info |
| R07 | 死代码 | 未用 import / 函数 / 变量 | warning |
| R08 | TODO/FIXME 堆积 | 累积计数 + 时效阈值 | info |
| R09 | 注释掉的代码块 | 正则 + 缩进启发 | warning |
| R10 | 魔法数字/字符串 | 未命名数字/字符串字面量 | info |

每条规则的阈值均可通过 TOML 配置覆盖。

### 4.2 CLI

- `codereviewer check <path>` — 扫描指定路径
- `codereviewer check <path> --format json` — JSON 输出
- `codereviewer check <path> --rules R01,R02` — 仅运行指定规则
- `codereviewer check <path> --severity error` — 仅显示指定级别及以上
- `codereviewer list-rules` — 列出所有可用规则
- 配置文件查找顺序：`--config` 参数 → 当前项目根 `.codereviewer.toml` → 内置默认

### 4.3 MCP Server

- 通过 stdio 传输
- 暴露工具：`review`（输入：路径；输出：结构化 finding 列表）
- 暴露工具：`list_rules`（输出：规则元信息列表）
- 兼容 MCP 协议规范，可被 opencode 等 MCP 客户端调用

### 4.4 配置

- 项目根 `.codereviewer.toml` 可覆盖：
  - 各规则阈值
  - 规则启用/禁用
  - 严重级调整
  - 文件排除 glob
  - 语言特定配置

## 5. 非功能性需求

### 5.1 性能

- 单文件扫描延迟 < 500ms（10K 行以内）
- 大型项目（10 万行）全量扫描 < 30s
- MCP 调用响应 < 2s（单文件）

### 5.2 可扩展性

- 新增规则的代码改动应局限在单一文件
- 新增语言支持应仅需添加 tree-sitter grammar 与语言特定规则映射
- 规则 trait 设计应允许未来 LLM 复核 hook 接入

### 5.3 可靠性

- 解析失败的文件跳过并记录，不中断整体扫描
- 规则 panic 隔离，单条规则失败不影响其他规则
- 零外部网络依赖（MVP 不调用任何 LLM API）

### 5.4 学习项目属性

- 代码应清晰体现 Rust 语言特性（ownership、trait、enum、error handling）
- 不为学习目的引入不必要的复杂度（仍遵循 Simplicity First）
- 关键设计决策在 `docs/design/design.md` 中说明

## 6. 验收标准

MVP 完成需同时满足：

1. `pwsh -Command "cargo build"` 零警告通过
2. `pwsh -Command "cargo test"` 全绿
3. 在人造问题代码 fixtures 上跑出全部 10 类 finding
4. 在真实 AI 生成代码上跑出有意义报告（人工验证）
5. opencode 配置该 MCP server 后能调用 `review` 工具并收到结构化结果
6. CLI JSON 输出可被脚本解析

## 7. 后续阶段路线

| 阶段 | 内容 |
|-----|------|
| Phase 2 | LLM 深审适配层：过度设计复核 + 不遵循需求检测 |
| Phase 2.5 | 重复代码块检测、圈复杂度 |
| Phase 3 | SARIF 输出、CI 集成（GitHub Actions） |
| Phase 4 | 更多语言（Go、Java、C++）、IDE 插件 |

## 8. 术语

- **finding**：一次检测发现，包含规则 ID、位置、严重级、消息
- **规则（rule）**：一个独立的检测逻辑单元
- **规则集（ruleset）**：所有可用规则的集合
- **配置（config）**：用户可调的规则参数与开关
