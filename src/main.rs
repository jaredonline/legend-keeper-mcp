mod lk;
mod prosemirror;
mod server;

use std::path::PathBuf;

use lk::io::read_lk_file;
use lk::store::WorldStore;
use rmcp::ServiceExt;
use server::LkServer;

fn resolve_worlds_dir() -> PathBuf {
    if let Ok(val) = std::env::var("LK_WORLDS") {
        return PathBuf::from(val);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".lk-worlds")
}

fn cmd_exports(worlds_dir: &PathBuf) {
    let exports_dir = worlds_dir.join("exports");
    if !exports_dir.exists() {
        println!("No exports directory found at {}", exports_dir.display());
        return;
    }

    let mut entries: Vec<_> = std::fs::read_dir(&exports_dir)
        .unwrap_or_else(|e| {
            eprintln!("Failed to read exports directory: {}", e);
            std::process::exit(1);
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "lk")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        println!("No .lk files in {}", exports_dir.display());
        return;
    }

    entries.sort_by_key(|e| e.file_name());

    println!(
        "{:<24} {:>10} {:>12}   {}",
        "WORLD", "RESOURCES", "SIZE", "EXPORTED"
    );
    println!("{}", "-".repeat(70));

    for entry in &entries {
        let path = entry.path();
        let world_name = path.file_stem().unwrap_or_default().to_string_lossy();

        let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let size_str = format_size(file_size);

        match read_lk_file(&path) {
            Ok(root) => {
                println!(
                    "{:<24} {:>10} {:>12}   {}",
                    world_name, root.resource_count, size_str, root.exported_at
                );
            }
            Err(e) => {
                println!("{:<24}   (error reading: {})", world_name, e);
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for subcommands
    if args.len() > 1 && args[1] == "exports" {
        let worlds_dir = if args.len() > 2 {
            PathBuf::from(&args[2])
        } else {
            resolve_worlds_dir()
        };
        cmd_exports(&worlds_dir);
        return Ok(());
    }

    let worlds_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        resolve_worlds_dir()
    };

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
