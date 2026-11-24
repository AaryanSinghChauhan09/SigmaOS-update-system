use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod updater;
mod error;
mod channel;
mod notifier;

use updater::UpdateManager;
use error::UpdateError;

#[derive(Parser)]
#[command(name = "sigmaos-update")]
#[command(about = "SigmaOS rolling release update system")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check for updates
    Check {
        /// Update channel
        #[arg(short, long, default_value = "stable")]
        channel: String,
        /// Force check
        #[arg(short, long)]
        force: bool,
    },
    /// Update system
    Update {
        /// Update channel
        #[arg(short, long, default_value = "stable")]
        channel: String,
        /// Dry run
        #[arg(short, long)]
        dry_run: bool,
        /// Force update
        #[arg(short, long)]
        force: bool,
    },
    /// Rollback to previous version
    Rollback {
        /// Target version
        #[arg(short, long)]
        version: Option<String>,
        /// List available versions
        #[arg(short, long)]
        list: bool,
    },
    /// Show update status
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },
    /// Configure update settings
    Config {
        /// Set update channel
        #[arg(short, long)]
        channel: Option<String>,
        /// Enable auto updates
        #[arg(short, long)]
        auto_update: Option<bool>,
        /// Set check interval (seconds)
        #[arg(short, long)]
        interval: Option<u32>,
    },
}

#[tokio::main]
async fn main() -> Result<(), UpdateError> {
    env_logger::init();
    
    let cli = Cli::parse();
    let update_manager = UpdateManager::new()?;
    
    match cli.command {
        Commands::Check { channel, force } => {
            let updates = update_manager.check_updates(&channel, force).await?;
            if updates.is_empty() {
                println!("No updates available");
            } else {
                println!("{} update(s) available:", updates.len());
                for update in updates {
                    println!("  {} -> {} ({})", update.current_version, update.new_version, update.channel);
                }
            }
        }
        Commands::Update { channel, dry_run, force } => {
            if dry_run {
                let updates = update_manager.check_updates(&channel, true).await?;
                println!("Dry run - {} update(s) would be installed:", updates.len());
                for update in updates {
                    println!("  {} -> {}", update.current_version, update.new_version);
                }
            } else {
                update_manager.update_system(&channel, force).await?;
                println!("System updated successfully");
            }
        }
        Commands::Rollback { version, list } => {
            if list {
                let versions = update_manager.list_versions()?;
                println!("Available versions:");
                for version in versions {
                    println!("  {}", version);
                }
            } else if let Some(version) = version {
                update_manager.rollback(&version)?;
                println!("Rolled back to version {}", version);
            } else {
                update_manager.rollback_to_previous()?;
                println!("Rolled back to previous version");
            }
        }
        Commands::Status { detailed } => {
            let status = update_manager.get_status()?;
            println!("Current version: {}", status.current_version);
            println!("Update channel: {}", status.channel);
            println!("Last check: {:?}", status.last_check);
            println!("Auto update: {}", status.auto_update);
            
            if detailed {
                println!("\nPending updates:");
                for update in status.pending_updates {
                    println!("  {} -> {}", update.current_version, update.new_version);
                }
            }
        }
        Commands::Config { channel, auto_update, interval } => {
            update_manager.update_config(channel, auto_update, interval)?;
            println!("Configuration updated");
        }
    }
    
    Ok(())
}
