//! MCP server for CodeReviewer: exposes review and list_rules tools.

use std::path::PathBuf;

use codereviewer_core::analyzer::Analyzer;
use codereviewer_core::config::Config;
use codereviewer_core::reporter::Report;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars;
use rmcp::tool;
use rmcp::tool_router;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReviewParams {
    /// Path to scan (file or directory)
    pub path: String,
}

#[derive(Clone)]
pub struct CodeReviewerServer;

#[tool_router(server_handler)]
impl CodeReviewerServer {
    #[tool(description = "Review code at the given path and return findings as JSON")]
    fn review(
        &self,
        Parameters(ReviewParams { path }): Parameters<ReviewParams>,
    ) -> Result<String, rmcp::ErrorData> {
        let rules = codereviewer_rules::all_rules();
        let analyzer = Analyzer::new(rules, Config::default());
        let result = analyzer.analyze_path(&PathBuf::from(&path));
        let report = Report { result };
        Ok(report.render_json())
    }

    #[tool(description = "List all available code review rules")]
    fn list_rules(&self) -> Result<String, rmcp::ErrorData> {
        let rules: Vec<_> = codereviewer_rules::all_rules()
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id(),
                    "name": r.name(),
                    "severity": r.severity().label(),
                })
            })
            .collect();
        Ok(serde_json::to_string_pretty(&rules).unwrap_or_default())
    }
}

pub async fn run_server() -> anyhow::Result<()> {
    let service = CodeReviewerServer.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
