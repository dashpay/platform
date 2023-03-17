use std::{panic, path::PathBuf};

use clap::{Parser, Subcommand};
use drive_abci::config::{FromEnv, PlatformConfig};

#[derive(Debug, Parser)]
#[command( author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Path to the config (.env) file.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath) ]
    config: Option<std::path::PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start server in foreground
    #[command()]
    Start {},
    /// Dump configuration
    ///
    /// WARNING: output can contain sensitive data!
    #[command()]
    Config {},
}

// #[derive(Debug, Args)]
// #[command(args_conflicts_with_subcommands = true)]
// struct StartArgs {
//     /// Path to data directory (GroveDB)
//     ///
//     /// Path to the data directory, containing transactions database.
//     #[arg(short, long, value_hint = clap::ValueHint::DirPath) ]
//     data: std::path::PathBuf,
// }

pub fn main() {
    let cli = Cli::parse();
    let config = load_config(&cli.config);

    match cli.command {
        Commands::Start {} => drive_abci::abci::server::start(&config).unwrap(),
        Commands::Config {} => dump_config(&config),
    }
}

fn dump_config(config: &PlatformConfig) {
    let serialized =
        serde_json::to_string_pretty(config).expect("failed to generate configuration");

    println!("{}", serialized);
}

fn load_config(config: &Option<PathBuf>) -> PlatformConfig {
    match config {
        Some(path) => {
            if let Err(e) = dotenvy::from_path(path) {
                panic!("cannot load config file {:?}: {}", path, e);
            }
        }
        None => {
            dotenvy::dotenv().expect("cannot load .env file");
        }
    };

    PlatformConfig::from_env().expect("cannot parse configuration file")
}
