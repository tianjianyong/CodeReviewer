# 案例：NavisworksTransport 规则校准实录

> 2026-08，CodeReviewer 首次在真实项目（337 文件 C# Navisworks 插件）上使用，
> 经目标项目 AI agent 四轮反馈完成规则校准。本案例记录过程与教训，作为后续规则调优的参照。

## 各轮数据

| 轮次 | 总 findings | error | 关键动作 |
|------|------------|-------|---------|
| 初始（修复前） | 12,248 | 407 | 首次扫描暴露大量规则 bug |
| 修复 + 调优配置 | 4,875 | 407 | R07/R03/R10 大修，R03 禁用，R02 阈值放宽 |
| 反馈第 1 轮 | 3,127 | 141 | R01 日志分档、R10 只报数字、R04 阈值 1、exclude_files、md 聚合 |
| 反馈第 2 轮 | 2,973 | 61 | R01 ex 引用分档、R23 catch_declaration 修复、宽泛 catch 精确化 |
| 反馈第 3 轮 | 2,984 | 40 | R01 三豁免一收紧、多行 bare catch 修复 |
| 反馈第 4 轮 | 40（收敛） | 40 | 抽查验证通过，反馈方主动喊停（收益递减） |

error 收敛路径：**407 → 141 → 61 → 40**，最终每条 error 都指向"彻底禁止 fallback"原则下的具体落点。

## 发现并修复的规则 bug

| # | bug | 影响 | 修复 |
|---|-----|------|------|
| 1 | R07 C#：`using` 是命名空间，单文件名字匹配原理不可行 | 几乎每文件误报（1029 条） | C# 移出 R07 支持列表 |
| 2 | R07 Python 点号导入取全名 | `import os.path` 必误报 | 取顶层包名 |
| 3 | R07 import 自身标识符污染"已使用"集合 | 普通未使用 import 从未被报 | 跳过 import 子树 |
| 4 | R19 Python 取循环变量而非可迭代对象 | 规则从未生效 | 从 "in" 之后取源表达式 |
| 5 | R23 异常变量检查看头部而非函数体 | `as e` 分支永不触发 | 函数体词边界匹配 |
| 6 | R23 C# `catch_declaration` 节点导致变量提取为空 | ex 豁免对 C# 从未生效（152 条几乎全误报） | 识别 catch_declaration 子节点 |
| 7 | R23 宽泛 catch 用子串匹配 | IOException 等具体异常被当宽泛 | 精确匹配基类 |
| 8 | R06 泛型 impl 匹配字符串错误 | `impl<T> Trait` 不识别 | 按 AST 提取 trait 名 |
| 9 | R03 C# 注释节点类型不识别 | 写了 /// 也报（2128→347） | 加入 `comment` 节点类型 |
| 10 | R10 中文 UI 文案当魔法字面量 | 5871 条 info 噪音 | 只报数字 + GetHashCode 种子豁免 |
| 11 | R01 多行 bare catch（catch 换行 {）不识别 | 真问题漏报（40 vs 5） | 按 catch 后首非空白字符判断 |
| 12 | R24 XML 命名空间 URI 误报 | 协议标识符当 URL | namespace/ ns 行白名单 |

## R01 判定终态（三轮演进结果）

```
catch (OperationCanceledException/TaskCanceledException)  → 不报（受控取消）
Try* 方法内的 catch + return false                       → 不报（.NET 惯例）
具体异常类型 + 语义返回值                                  → info
宽泛 catch + 硬编码默认返回 + 无日志 + 不引用 ex            → error（唯一 error 档）
其余（有日志 / 返回值携带 ex）                             → info
```

## 方法论沉淀

1. **真实项目试用比造 fixture 更能暴露 bug**——四轮反馈揪出 12 个规则 bug，大部分 fixture 无法复现（命名空间语义、UI 文案、XML 命名空间 URI 等）
2. **反馈方的预期数字是验证信号**——agent 预估 R01 error ~25，实测 5，数字对不上→追出多行 bare catch bug。校准循环里"实测数 vs 预期数"应显式对比
3. **规则分档的价值**——同一规则按上下文分 severity（error/info/不报）比单一判定更接近真实使用；error 档应保持"每一条都可执行"
4. **校准有终点**——反馈方主动喊停（哨兵值 -1/0/null 识别收益递减）。尊重使用方的判断，不再为了数字好看继续调
5. **可配置性兜底**——无法通用的项目约定（文档在 docs/、测试基础设施路径）交给 `.codereviewer.toml`（禁用规则、调阈值、exclude_files），而不是写进规则
