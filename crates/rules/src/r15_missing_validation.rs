//! R15: 边界输入未校验检测。
//!
//! 启发式：函数（HTTP handler / pub fn / 公开 API）参数被直接索引 arr[i]
//! 或除以 len()，且函数体内无 is_empty/len() > 边界守护。
//! happy-path 幻觉——AI 处理完美输入，忽略空/越界。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct MissingInputValidation;

impl Rule for MissingInputValidation {
    fn id(&self) -> &'static str {
        "R15"
    }
    fn name(&self) -> &'static str {
        "missing-input-validation"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &'static [Language] {
        &[
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::TypeScriptTsx,
            Language::CSharp,
            Language::Java,
        ]
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError> {
        let function_kinds = function_kinds(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !function_kinds.contains(&node.kind()) {
                return;
            }
            if !is_boundary_function(&node, ctx) {
                return;
            }
            let params = collect_param_names(&node, ctx);
            if params.is_empty() {
                return;
            }
            let body_text = node_text(&node, ctx.source);
            let has_guard = has_validation_guard(&body_text, ctx.language);
            if has_guard {
                return;
            }
            // 检测危险操作：除以 len() / 索引 param[i]
            let danger = find_dangerous_op(&node, &params, ctx);
            if let Some(desc) = danger {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R15",
                    rule_name: "missing-input-validation",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "边界函数对参数 {} 无空值/越界校验即进行危险操作（{}） | boundary function performs {} on param {} without empty/bounds validation",
                        desc.0, desc.1, desc.1, desc.0
                    ),
                    snippet: None,
                });
            }
        });

        Ok(findings)
    }
}

fn function_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["function_item"],
        Language::Python => &["function_definition"],
        Language::TypeScript | Language::TypeScriptTsx => &["function_declaration", "method_definition"],
        Language::CSharp => &["method_declaration"],
        Language::Java => &["method_declaration"],
    }
}

fn is_boundary_function(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let text = node_text(node, ctx.source);
    // Rust pub fn（API 面）
    if ctx.language == Language::Rust && text.contains("pub fn") {
        return true;
    }
    // HTTP handler 信号
    let handler_signals = [
        "@app.route", "@app.get", "@app.post", "@app.put", "@app.delete",
        "@router.", "@Get", "@Post", "@Put", "@Delete", "@RequestMapping",
        "[HttpGet", "[HttpPost", "[Route",
        "@GetMapping", "@PostMapping",
        "req.", "request.", "req:", "request:",
        "handler", "Handler",
    ];
    if handler_signals.iter().any(|s| text.contains(s)) {
        return true;
    }
    // Python 模块级 def（column == 0）且接受外部输入
    if ctx.language == Language::Python {
        let pos = node.start_position();
        if pos.column == 0 && text.contains("def ") {
            return true;
        }
    }
    false
}

fn collect_param_names(node: &tree_sitter::Node, ctx: &AnalysisContext) -> Vec<String> {
    let param_kind = match ctx.language {
        Language::Rust => "parameters",
        Language::Python => "parameters",
        Language::TypeScript | Language::TypeScriptTsx => "formal_parameters",
        Language::CSharp => "parameter_list",
        Language::Java => "formal_parameters",
    };
    let mut params = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == param_kind {
            let mut inner = child.walk();
            for p in child.children(&mut inner) {
                if is_parameter_node(&p, ctx.language) {
                    let name = node_text(&p, ctx.source);
                    let clean = name.split([':', ' ', '=']).next().unwrap_or(name);
                    params.push(clean.to_string());
                }
            }
        }
    }
    params
}

fn is_parameter_node(node: &tree_sitter::Node, lang: Language) -> bool {
    match lang {
        Language::Rust => node.kind() == "parameter",
        Language::Python => node.kind() == "identifier" || node.kind() == "typed_parameter",
        Language::TypeScript | Language::TypeScriptTsx => {
            node.kind() == "required_parameter" || node.kind() == "optional_parameter"
        }
        Language::CSharp => node.kind() == "parameter",
        Language::Java => node.kind() == "formal_parameter",
    }
}

fn has_validation_guard(body: &str, _lang: Language) -> bool {
    let guards = [
        "is_empty()", "!is_empty", ".is_empty", "len() == 0", "len() >",
        "len() <", "is_none()", "is_some()", "is_ok()", "is_err()",
        "is_empty", "len(x) ==", "len(x) >", "if not ", "if len(",
        ".length ==", ".length >", ".length <", ".length === 0",
        "isNullOr", "isUndefined", "isNil", "if (!", "if (x ==",
        "isNullOrEmpty", "isNullOrWhiteSpace",
    ];
    guards.iter().any(|g| body.contains(g))
}

fn find_dangerous_op(
    func_node: &tree_sitter::Node,
    params: &[String],
    ctx: &AnalysisContext,
) -> Option<(String, String)> {
    let body_text = node_text(func_node, ctx.source);
    // 除以 param.len() / param.length
    for p in params {
        let len_forms = [
            format!("{}.len()", p),
            format!("{}.length", p),
            format!("len({})", p),
        ];
        for lf in &len_forms {
            if body_text.contains(&format!("/ {}", lf)) || body_text.contains(&format!("/{}", lf)) {
                return Some((p.clone(), "除以长度".to_string()));
            }
        }
        // 索引 param[i] / param[i+1]
        let index_form = format!("{}[", p);
        if body_text.contains(&index_form) {
            // 排除 param[0] / param[-1] 这类常量索引
            if has_variable_index(func_node, p, ctx) {
                return Some((p.clone(), "变量索引".to_string()));
            }
        }
    }
    None
}

fn has_variable_index(func_node: &tree_sitter::Node, param: &str, ctx: &AnalysisContext) -> bool {
    let prefix = format!("{}[", param);
    let mut found = false;
    walk(*func_node, &mut |n| {
        if found {
            return;
        }
        let text = node_text(&n, ctx.source);
        if !text.starts_with(&prefix) {
            return;
        }
        // [i] / [i+1] / [idx] / [i-1] 等非常量
        let inner = text
            .get(prefix.len()..)
            .and_then(|s| s.split(']').next())
            .unwrap_or("");
        let trimmed = inner.trim();
        // 常量索引：纯数字、0、-1
        let is_const = trimmed.parse::<i64>().is_ok() || trimmed == "0";
        if !is_const && !trimmed.is_empty() {
            found = true;
        }
    });
    found
}

fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    source.get(node.start_byte()..node.end_byte()).unwrap_or("")
}

fn walk<F: FnMut(tree_sitter::Node)>(node: tree_sitter::Node, visit: &mut F) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        visit(n);
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
}
