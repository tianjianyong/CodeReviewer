#[tokio::main]
async fn main() -> anyhow::Result<()> {
    codereviewer_mcp::run_server().await
}
