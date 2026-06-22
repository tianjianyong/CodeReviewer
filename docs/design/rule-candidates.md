# AI 代码审计候选规则（R11–R28）

> 来源：针对 AI vibe coding 代码缺陷的网络调研。主要参考：
> - GitHub Copilot 官方 review 指南 https://docs.github.com/en/copilot/tutorials/review-ai-generated-code
> - ShiftAsia 6 层 checklist https://shiftasia.com/column/how-to-review-ai-generated-code-the-complete-developers-guide/
> - WonderingAboutAI 11 类 vibe coding bug https://wonderingaboutai.substack.com/p/a-field-guide-to-11-common-vibe-coding
> - Veracode 2025 GenAI 代码安全报告（AI 86–88% 生成不安全加密/日志代码）
> - Krun 工程 checklist https://krun.pro/ai-generated-code/
>
> 现有规则：R01–R10（见 README）。下表为**候选**规则，按优先级排序。
> 标注「✅ 已实现」的为当前版本已落地；其余为后续迭代备选。

## 第一梯队（AI 特有，人眼难抓）

| ID | 规则名 | 严重级 | 抓什么 | 检测思路 | 状态 |
|----|--------|--------|--------|----------|------|
| R11 | `hallucinated-api-call` | error | 调用依赖**实际锁定版本**里不存在的方法/参数/枚举（LLM 训练数据混了多版本 SDK） | 解析 `Cargo.lock`/`package-lock`/`requirements.txt`，比对该版本导出符号集 | 备选 |
| R12 | `invented-import` | error | import 了 manifest 没声明、或包根本不存在于 registry 的依赖（slopsquatting 风险） | 逐条 import 对照 manifest + registry API 校验 | 备选 |
| R17 | `skipped-or-deleted-test` | warning | PR 改了实现文件却**删除/跳过**测试（`#[ignore]`/`pytest.mark.skip`/`.skip()`），而不是修测试 | diff-aware：检测被删的 test fn、新增 skip marker | 备选 |

## 第二梯队（高危安全，AI 触发率显著高于人类）

| ID | 规则名 | 严重级 | 抓什么 | 数据支撑 / 检测思路 | 状态 |
|----|--------|--------|--------|----------------------|------|
| R13 | `string-interpolated-sql` | error | f-string/format/`+` 拼 SQL 喂给 execute | Nucamp: AI 生成的查询 ~40% 有 SQLi；AST 找含 SQL 关键字的插值字符串流入 execute | 备选 |
| R14 | `hardcoded-secret` | error | 源码里硬编码 API key/token/私钥 | Lovable 事件暴露 150 万 key；正则匹配已知前缀 `sk-`/`ghp_`/`AKIA`/`xoxb-`/`AIza`/`BEGIN PRIVATE KEY` + 高熵串赋给 secret 命名变量 | ✅ 已实现 |
| R21 | `insecure-crypto-default` | error | ECB 模式、SHA-256 直哈密码、`Math.random()` 做 IV/nonce | Veracode: AI 88% 生成不安全日志/加密；AST 找 ECB/MD5/裸 SHA-256 哈密码/Math.random 喂 token | 备选 |

## 第三梯队（AI 偏好导致的逻辑/质量缺陷）

| ID | 规则名 | 严重级 | 抓什么 | 检测思路 | 状态 |
|----|--------|--------|--------|----------|------|
| R15 | `missing-input-validation-at-boundary` | warning | HTTP handler/CLI 入口对外部输入直接 `arr[i]`/`/ len()` 无空值/越界检查（happy-path 幻觉） | 识别边界函数（路由注解/`pub fn` API 面），检测未守护的索引/除长度 | ✅ 已实现 |
| R16 | `self-validating-test` | warning | 测试断言实现内部状态而非行为（与 R04/R05 互补——断言够多但仍只测了 impl 视角） | test fn 名匹配 `^test_<method>$` 无行为后缀；访问私有成员 `_foo`/`__foo` | ✅ 已实现 |
| R18 | `async-missing-await` | error | async 调用未 await 就用其副作用；async fn 写共享可变状态无锁 | 同文件内收集 async fn 名，找未 await 的调用结果被丢弃 | ✅ 已实现 |
| R19 | `n-plus-one-query` | warning | ORM 循环里访问关系字段触发每行一次查询（AI "signature anti-pattern"） | `for x in <queryset>` 体内访问 `x.<关系字段>`，源无 `select_related`/`prefetch_related`/`include` | ✅ 已实现 |
| R20 | `resource-leak` | warning | `open`/`connect`/`addEventListener` 无配对 close/remove，尤其循环里挂监听 | open/connect 调用不在 with/try-finally 且同函数无 .close()；循环内 addEventListener 无 removeEventListener | ✅ 已实现 |
| R23 | `wrong-error-type-propagation` | warning | 错误**有**传播但类型丢信息（`except Exception: return 500` 在本应返 401/403/404 的 handler 里） | 宽泛 catch 体内 return 固定状态码/泛型错误且不引用异常变量 | ✅ 已实现 |
| R24 | `hardcoded-path-or-url` | warning | 源码里 `/Users/tellme/...`、`http://localhost:3000`（把 prompt 里的本地环境复制进代码） | 正则匹配绝对路径/带 host 的 URL 字面量，非测试文件，非 env::var 包裹 | ✅ 已实现 |

## 第四梯队（次重要但值得有）

| ID | 规则名 | 严重级 | 抓什么 | 检测思路 | 状态 |
|----|--------|--------|--------|----------|------|
| R22 | `context-blind-convention-violation` | info | 新代码用模块里的少数派写法（`.unwrap()` 出现在全用 `?` 的文件里） | 按模块统计多数构造（错误处理/命名/日志宏），diff 中新代码用 <20% 占比的构造 | 备选 |
| R25 | `log-injection-unsanitized` | warning | 外部输入直接进 log，未剥离 `\r\n` | log 调用参数可追溯到 req param/header 且未过 sanitizer | 备选 |
| R26 | `missing-rate-limit-handling` | info | 调外部 API 不处理 429 / 不读 `Retry-After` | HTTP client 调用同函数内无 `status == 429` 分支 / 无 Retry-After 读取 | 备选 |
| R27 | `comment-code-mismatch` | info | 注释动词与下一句实际操作不符（`// sort 升序` 上方是 `.rev()`） | 提取行注释动词，对照下一语句 AST 根；完整覆盖需 LLM judge | 备选 |
| R28 | `overly-defensive-handling` | info | R01 的反面：给类型系统已保证不可失败的操作套 try/catch/unwrap_or | `Some(...).unwrap_or(...)`/`Ok(...).unwrap_or(...)`；try 包纯算术 | ✅ 已实现 |

## 实现优先级建议

1. **R11 + R12**（AI 独有、人眼抓不到）——需读 lockfile + 查 registry，工程量较大
2. **R13 / R14 / R21**（安全三件套）——AI 文档高触发率
3. **R16 + R17**（测试质量）——补齐 R04/R05 未覆盖的虚假信心问题

## 已实现规则的已知局限

- **R15**：MVP 仅检测「参数索引/除长度无守护」的常见模式，不识别框架级 validator（可能误报）
- **R18**：仅做单文件内 async fn 名匹配，无跨文件类型信息（漏报外部 async 调用）
- **R19**：无 schema 信息，靠 queryset 信号词启发式，可能误报普通循环
- **R20**：Rust 因 RAII 不适用；Python/JS 检测同函数内 close 配对，跨函数释放漏报
- **R23**：仅检测宽泛 catch 返回固定码且不引用异常变量的模式
- **R28**：仅检测 `Some/Ok` 字面量上的 unwrap_or，复杂不可失败路径漏报
