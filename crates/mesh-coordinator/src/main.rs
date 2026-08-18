//! BourbakiMesh Distributed Mesh Coordinator CLI.

use clap::Parser;
use mesh_coordinator::server::MeshCoordinatorServer;

#[derive(Parser, Debug)]
#[command(author, version, about = "BourbakiMesh Distributed Proof Coordinator")]
struct Args {
    /// Socket bind address for WebSocket RPC server
    #[arg(short, long, default_value = "127.0.0.1:9001")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    println!("=== BourbakiMesh Distributed Mesh Coordinator ===");
    println!("Binding WebSocket RPC server to {}", args.addr);

    let server = MeshCoordinatorServer::new();
    server.run(&args.addr).await?;

    Ok(())
}
