use rmcp::ServiceExt;
use rmcp::transport::stdio;
use tracing_subscriber::EnvFilter;

mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP servers must never write non-protocol data to stdout.
    // All logging goes to stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .init();

    let server = server::FallowMcp::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
