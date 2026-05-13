#[allow(unused_imports)]
pub use zeroclaw_runtime::peers::*;

use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;

/// Return the path for a specific peer file in the workspace.
fn peer_file_path(workspace_dir: &std::path::Path, name: &str) -> PathBuf {
    workspace_dir.join(format!("peer-{name}.md"))
}

/// Return the path for the default peer file in the workspace.
fn default_peer_file_path(workspace_dir: &std::path::Path) -> PathBuf {
    workspace_dir.join("peer.md")
}

pub fn handle_command(command: crate::PeerCommands, config: &crate::config::Config) -> Result<()> {
    let workspace_dir = &config.workspace_dir;
    match command {
        crate::PeerCommands::List => {
            let peers = load_peers(workspace_dir);
            let default_exists = default_peer_file_path(workspace_dir).exists();

            if peers.is_empty() && !default_exists {
                println!("No peer profiles found.");
                println!();
                println!("  Create default: echo '# My Preferences' > ~/.zeroclaw/workspace/peer.md");
                println!(
                    "  Create specific: echo '# Alice' > ~/.zeroclaw/workspace/peer-alice.md"
                );
                println!();
                println!("  Or use: zeroclaw peer add <name>");
            } else {
                if default_exists {
                    println!("Default peer profile: peer.md");
                }
                if !peers.is_empty() {
                    println!("Specific peer profiles ({}):", peers.len());
                    println!();
                    for peer in &peers {
                        println!(
                            "  {} — {}",
                            console::style(&peer.name).white().bold(),
                            peer.description
                        );
                        if !peer.tags.is_empty() {
                            println!("    Tags: {}", peer.tags.join(", "));
                        }
                    }
                }
            }
            println!();
            Ok(())
        }
        crate::PeerCommands::Show { name } => {
            let peer = load_peer(workspace_dir, &name)
                .ok_or_else(|| anyhow::anyhow!("Peer profile not found: {name}"))?;

            println!(
                "{} {}",
                console::style(&peer.name).white().bold(),
                console::style(format!("— {}", peer.description)).dim()
            );
            if !peer.tags.is_empty() {
                println!("  Tags: {}", peer.tags.join(", "));
            }
            if let Some(ref loc) = peer.location {
                println!("  Location: {}", loc.display());
            }
            println!();
            if !peer.content.trim().is_empty() {
                println!("{}", peer.content);
            }
            println!();
            Ok(())
        }
        crate::PeerCommands::Add { name } => {
            let md_path = peer_file_path(workspace_dir, &name);
            if md_path.exists() {
                println!(
                    "  Peer profile '{}' already exists at {}",
                    name,
                    md_path.display()
                );
                println!("  Edit it directly: {}", md_path.display());
                return Ok(());
            }

            let template = format!(
                "# Peer: {name}\n\n\
                 ## Preferences\n\n\
                 - Add your preferences here.\n\
                 - Example: 喜欢简单的内容，不喜欢复杂的原理\n\
                 - Example: 喜欢从核心概念开始了解一个事物\n\
                 - Example: 不喜欢别人用太多的比喻和比拟来介绍事物\n\n\
                 ## Notes\n\n\
                 - Any additional context about this peer.\n"
            );
            std::fs::write(&md_path, &template)?;
            println!(
                "  {} Peer profile '{}' created at {}",
                console::style("✓").green().bold(),
                name,
                md_path.display()
            );
            println!();
            println!("  Edit the file to customize the peer's preferences.");
            Ok(())
        }
        crate::PeerCommands::Remove { name } => {
            if name.contains("..") || name.contains('/') || name.contains('\\') {
                anyhow::bail!("Invalid peer name: {name}");
            }

            let peer_path = peer_file_path(workspace_dir, &name);
            let canonical_workspace = workspace_dir
                .canonicalize()
                .unwrap_or_else(|_| workspace_dir.to_path_buf());
            if let Ok(canonical_peer) = peer_path.canonicalize() {
                if !canonical_peer.starts_with(&canonical_workspace) {
                    anyhow::bail!("Peer path escapes workspace directory: {name}");
                }
            }

            if !peer_path.exists() {
                anyhow::bail!("Peer profile not found: {name}");
            }

            print!(
                "Remove peer profile '{}'? [y/N] ",
                name
            );
            std::io::stdout().flush()?;
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }

            std::fs::remove_file(&peer_path)?;
            println!(
                "  {} Peer profile '{}' removed.",
                console::style("✓").green().bold(),
                name
            );
            Ok(())
        }
    }
}
