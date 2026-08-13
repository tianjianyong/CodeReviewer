//! 共享 AST 工具：节点遍历、源码切片、行文本。

use tree_sitter::Node;

/// 迭代遍历节点（含自身与所有后代）。
pub fn walk<F: FnMut(Node)>(node: Node, visit: &mut F) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        visit(n);
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
}

/// 节点对应的源码文本切片。
pub fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    source.get(node.start_byte()..node.end_byte()).unwrap_or("")
}

/// 指定行的源码文本（0 基）。
pub fn line_of(source: &str, row: usize) -> &str {
    source.lines().nth(row).unwrap_or("")
}

/// 词边界包含：避免 "e" 命中 "return"、"error"。
pub fn contains_word(text: &str, word: &str) -> bool {
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let mut rest = text;
    while let Some(pos) = rest.find(word) {
        let end = pos + word.len();
        let before_ok = pos == 0
            || !rest[..pos]
                .chars()
                .last()
                .map(is_word_char)
                .unwrap_or(false);
        let after_ok = end >= rest.len()
            || !rest[end..]
                .chars()
                .next()
                .map(is_word_char)
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        rest = &rest[pos + 1..];
    }
    false
}
