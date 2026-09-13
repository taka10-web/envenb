use std::sync::Arc;

use envfish_mcp::McpServer;

use crate::commands::Ctx;

/// `envfish mcp`: serve MCP over stdio. All logging goes to stderr so stdout stays
/// a clean JSON-RPC stream. The goldfish never swims here.
pub async fn run(ctx: Ctx, client: &str, kind: &str) -> anyhow::Result<()> {
    let core = Arc::new(ctx.app);
    let server = McpServer::new(core, client, kind).await?;
    tracing::info!(client = %server.client().name, "mcp server ready on stdio");
    server.serve_stdio().await?;
    Ok(())
}
