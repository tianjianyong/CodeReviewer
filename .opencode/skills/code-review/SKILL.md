---
name: code-review
description: Review code for AI-generated code quality issues using the CodeReviewer MCP server. Use after writing or modifying code to catch over-engineering, error masking, missing docs, shallow tests, and other common AI code problems.
---

# Code Review Skill

When asked to review code, or after generating/modifying code, use the CodeReviewer MCP server to run automated analysis.

## How to use

1. Call the `review` MCP tool with the path to the file or directory to scan.
2. Read the JSON findings in the response.
3. Summarize the findings for the user, grouped by severity:
   - **error**: Must fix before merge (R01 fallback masks error)
   - **warning**: Should fix (R02 bloat, R03 missing doc, R04 simple test, R07 dead code, R09 commented code)
   - **info**: Consider fixing (R06 over-engineering, R08 TODO, R10 magic number)
4. For each finding, cite `file:line:column` and the rule message.
5. Suggest concrete fixes for the most severe findings.

## Available rules

| ID | Name | Severity | What it catches |
|----|------|----------|-----------------|
| R01 | fallback-masks-error | error | catch/unwrap_or swallowing errors |
| R02 | structural-bloat | warning | long functions, deep nesting, too many params |
| R03 | missing-doc | warning | public items without doc comments |
| R04 | simple-unit-test | warning | tests with too few assertions |
| R05 | shallow-integration-test | warning | integration tests only checking status codes |
| R06 | over-engineering | info | single-impl traits, excessive generics |
| R07 | dead-code | warning | unused imports |
| R08 | todo-fixme-accumulation | info | TODO/FIXME markers |
| R09 | commented-out-code | warning | commented-out code blocks |
| R10 | magic-number | info | magic literals |
