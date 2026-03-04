mod lk;
mod prosemirror;
mod server;

use std::path::PathBuf;

use lk::store::WorldStore;
use rmcp::ServiceExt;
use server::LkServer;

fn resolve_worlds_dir() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    if let Ok(val) = std::env::var("LK_WORLDS") {
        return PathBuf::from(val);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".lk-worlds")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let worlds_dir = resolve_worlds_dir();

    if !worlds_dir.exists() {
        std::fs::create_dir_all(&worlds_dir)?;
    }

    eprintln!("Legend Keeper MCP server");
    eprintln!("Worlds directory: {}", worlds_dir.display());

    let store = WorldStore::load(&worlds_dir)?;

    let worlds = store.list_worlds();
    eprintln!("Loaded {} world(s):", worlds.len());
    for w in &worlds {
        eprintln!(
            "  {} — {} resources, {} calendars",
            w.name, w.resource_count, w.calendar_count
        );
    }

    // Start file watcher (keep handle alive)
    let _watcher = store.start_watcher()?;

    let server = LkServer::new(store);
    let running = server.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;

    Ok(())
}
