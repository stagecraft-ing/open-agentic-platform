// The web binary shares command modules with the Tauri desktop app.
// Most code is invoked via Tauri's generate_handler![] in the main binary
// and appears dead here. Suppress these false positives.
#![allow(dead_code)]

use clap::Parser;

mod checkpoint;
mod claude_binary;
mod governed_claude;
mod commands;

/// `opc-web` binary has no Tauri sidecar lifecycle; [`SidecarState::axiomregent_port`] stays `None`.
pub mod sidecars {
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    pub struct SidecarState {
        pub axiomregent_port: Arc<Mutex<Option<u16>>>,
    }
}
mod process;
mod types;
mod utils;
mod web_server;

#[derive(Parser)]
#[command(name = "opc-web")]
#[command(about = "Opcode Web Server - Access Opcode from your phone")]
struct Args {
    /// Port to run the web server on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Host to bind to (0.0.0.0 for all interfaces)
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();

    println!("🚀 Starting Opcode Web Server...");
    println!(
        "📱 Will be accessible from phones at: http://{}:{}",
        args.host, args.port
    );

    if let Err(e) = web_server::start_web_mode(Some(args.port)).await {
        eprintln!("❌ Failed to start web server: {}", e);
        std::process::exit(1);
    }
}
