use std::io::{self, BufRead, Write};

use clap::Parser;
use recallgate_mcp::server::McpServer;

/// Recall Gate MCP server (stdio transport).
#[derive(Debug, Parser)]
#[command(name = "recallgate-mcp", version, about)]
struct Cli {}

fn main() -> io::Result<()> {
    let _cli = Cli::parse();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::default();

    for line in stdin.lock().lines() {
        let line = line?;
        if let Some(response) = server.handle_line(&line) {
            stdout.write_all(response.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
