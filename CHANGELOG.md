# Changelog

本项目的所有重要变更都记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。当前版本号见 [VERSION.md](VERSION.md)。

## [Unreleased]

### 修复（Fixed）

- 排除规则按子串匹配误伤正常文件（`combine.py` 被 "bin" 模式命中），改为按路径组件名匹配
- 扫描不存在的路径静默成功（0 findings + exit 0），现在报错并非零退出
- CLI `--severity`/`--rules`/`--format` 无效值静默忽略，现在报错退出
- 退出码恒为 0，现在有 error 级 finding 时 exit 1（CI 门禁可用）
- R07 死代码：Python 点号导入（`import os.path`）误报；TS 默认/命名空间导入漏报；Java 通配符导入误报；import 语句自身标识符污染"已使用"集合导致普通未使用 import 从未被报；C# 移出支持列表（using 是命名空间，单文件名字匹配原理上不可行，此前几乎每文件误报）
- R19 N+1：Python 版取循环变量而非可迭代对象，规则从未生效
- R23 错误类型传播：异常变量检查在 catch 头部而非函数体；Python `except Exception as e` 的 `as_pattern` 结构导致变量提取为空，带异常变量的分支从未生效
- R06 过度设计：泛型 impl 匹配字符串错误（`impl<T> Trait` 不识别），改为按 AST 提取 trait 名并排除固有 impl
- R08 TODO：C# comment 节点种类混入 `extern_alias_directive`
- R03 文档缺失：C# 注释节点类型是 `comment`（不分 line/block），has_doc_comment 未识别，写了 /// 的公开项照样被报（实测 2128 条中 1781 条为误报）；现已识别，附 C# 单测
- R10 魔法字面量：跳过含 CJK 汉字的字符串（UI 文案是人类可读文本，非魔法常量），数字与 ASCII 编码/协议串仍报
- 解析失败的文件在报告中不可见，现在文本/JSON 都输出 `parse_errors` 与跳过计数

### 变更（Changed）

- 规则校准（实测驱动）：R01 catch+return 按有无日志拆两档（无日志 error / 有日志 info 防御式处理）；R10 只报数字字面量（字符串误报率高，已移除）并跳过 GetHashCode/hashCode 哈希种子；R04 默认阈值降为 1（只报无断言测试）；R09 说明性注释（注意/说明/TODO 等）不计入注释代码块；规则级 `exclude_files` 文件排除（如测试基础设施退出 R24）

- 抽取共享 `core::ast` 工具（`walk`/`node_text`/`line_of`）与 `Finding::new`（自动带 snippet），规则层净减 334 行
- 文件只读一次；`.gitignore` 模式继承到子目录；排除模式每次扫描只计算一次
- MCP server 与 CLI 共用 `.codereviewer.toml` 配置加载（此前 MCP 恒用默认配置）；配置发现锚定扫描路径（向上一层查找，回退当前目录）
- JSON 输出新增 `snippet`、`skipped`、`parse_errors` 字段
- 新增 `--format md` 汇总报告（统计 + error 级问题分组 + 修复建议 + 重灾区文件）与 `--output <file>` 报告落盘（详细报告用 `--format json --output`）
- 新增 `--baseline <上次报告.json>` 增量对比：按 (file, line, rule) 过滤已存在问题，只报新增；退出码只反映新增 error（CI 增量门禁）
- 规则声明语言与实现对齐：R18/R19/R20 仅 Python/TS/TSX，R23 不含 Rust，R28 仅 Rust/TS/TSX
- 文件级并行分析（rayon），输出顺序保持确定性
- r02 嵌套深度改为迭代实现（消除深 AST 递归栈溢出风险）
- clippy 告警清零；新增 CI workflow（fmt + clippy + test + release build）
- 新增 MIT LICENSE；MCP 配置命令改为跨平台的 `cargo run` 形式

### 移除（Removed）

- `core::llm` Phase 2 占位模块（无实现、无调用者）
- `RuleError::Failed`（无任何规则构造过）
- `core::VERSION` 死常量（版本由 `Cargo.toml` + `VERSION.md` 维护，测试保证一致）

## [0.1.0] - 2026-06-22

首个 MVP 版本。

### 新增（Added）

- CLI：`check <path>`（`--format`/`--rules`/`--severity`）与 `list-rules` 子命令
- MCP server（stdio）：`review` 与 `list_rules` 工具
- tree-sitter 多语言解析：Rust、Python、TypeScript/TSX、C#、Java
- 基础规则 R01–R10：回退掩盖、结构臃肿、文档缺失、测试简单、浅集成测试、过度设计、死代码、TODO 堆积、注释代码、魔法数字
- AI 代码专项规则 R14/R15/R16/R18/R19/R20/R23/R24/R28（候选清单见 `docs/design/rule-candidates.md`）
- TOML 配置（`.codereviewer.toml`）：规则启停、阈值覆盖、项目排除目录
- 内置默认排除目录与 `.gitignore` 读取
- 终端彩色/文本与 JSON 两种报告格式
