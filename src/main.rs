mod args;
mod assets;
mod data;
mod downloader;
mod net;
mod physics;
mod player;
mod renderer;
mod ui;
mod window;
mod world;

use std::sync::Arc;

use clap::Parser;

use net::connection::ConnectArgs;

const DEFAULT_VERSION: &str = "1.21.11";

fn main() {
    env_logger::init();

    let args = args::LaunchArgs::parse();
    let data = data::DataDir::resolve(args.game_dir.as_deref(), args.assets_dir.as_deref());

    if let Err(e) = data.ensure_dirs() {
        log::error!("Failed to create data directories: {e}");
        std::process::exit(1);
    }

    log::info!("Data directory: {}", data.root.display());

    let rt = Arc::new(tokio::runtime::Runtime::new().expect("failed to create tokio runtime"));
    let version = args
        .version
        .as_deref()
        .unwrap_or(DEFAULT_VERSION)
        .to_string();

    let connection = if let Some(ref server) = args.server {
        let connect_args = ConnectArgs {
            server: server.clone(),
            username: args.username.clone().unwrap_or_else(|| "Steve".into()),
            uuid: args
                .uuid
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(uuid::Uuid::nil),
            access_token: args.access_token.clone(),
            view_distance: 12,
        };

        Some(net::connection::spawn_connection(&rt, connect_args))
    } else {
        None
    };

    if let Err(e) = window::run(connection, data, version, rt) {
        log::error!("Fatal: {e}");
        std::process::exit(1);
    }
}
