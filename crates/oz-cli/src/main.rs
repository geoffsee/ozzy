mod client;
mod config;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use oz_core::{Project, SecretMeta, SecretValue};
use std::io::{self, Read};

use client::ApiClient;
use config::Config;

#[derive(Parser)]
#[command(name = "oz", about = "oz secrets CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    Login {
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        api_url: Option<String>,
    },
    Logout,
}

#[derive(Subcommand)]
enum ProjectCommands {
    List,
}

#[derive(Subcommand)]
enum SecretsCommands {
    List {
        #[arg(long)]
        project: String,
    },
    Get {
        key: String,
        #[arg(long)]
        project: String,
    },
    Set {
        key: String,
        #[arg(long)]
        project: String,
        value: Option<String>,
        #[arg(long)]
        from_stdin: bool,
    },
    Delete {
        key: String,
        #[arg(long)]
        project: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Auth { command } => handle_auth(command),
        Commands::Project { command } => handle_project(command),
        Commands::Secrets { command } => handle_secrets(command),
    }
}

fn handle_auth(command: AuthCommands) -> Result<()> {
    match command {
        AuthCommands::Login { api_key, api_url } => {
            let mut cfg = Config::load().unwrap_or_default();
            if let Some(url) = api_url {
                cfg.api_url = url;
            }
            if let Some(key) = api_key.or_else(|| std::env::var("OZ_API_KEY").ok()) {
                cfg.api_key = Some(key);
            } else if cfg.api_key.is_none() {
                bail!("provide --api-key or set OZ_API_KEY");
            }
            cfg.save()?;
            println!("Saved credentials to {}", Config::path()?.display());
            Ok(())
        }
        AuthCommands::Logout => {
            Config::clear()?;
            println!("Logged out");
            Ok(())
        }
    }
}

fn handle_project(command: ProjectCommands) -> Result<()> {
    let client = ApiClient::from_env()?;
    match command {
        ProjectCommands::List => {
            let projects: Vec<Project> = client.get("/v2/projects")?;
            if projects.is_empty() {
                println!("No projects");
            } else {
                for p in projects {
                    println!("{} ({})", p.slug, p.name);
                }
            }
            Ok(())
        }
    }
}

fn handle_secrets(command: SecretsCommands) -> Result<()> {
    let client = ApiClient::from_env()?;
    match command {
        SecretsCommands::List { project } => {
            let secrets: Vec<SecretMeta> = client.post(
                "/v2/secrets/list",
                &serde_json::json!({ "project": project }),
            )?;
            for s in secrets {
                println!("{} (v{})", s.key_name, s.version);
            }
            Ok(())
        }
        SecretsCommands::Get { key, project } => {
            let secret: SecretValue = client.post(
                "/v2/secrets/read",
                &serde_json::json!({ "project": project, "key": key }),
            )?;
            println!("{}", secret.value);
            Ok(())
        }
        SecretsCommands::Set {
            key,
            project,
            value,
            from_stdin,
        } => {
            let val = if from_stdin {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf.trim_end_matches('\n').to_string()
            } else {
                value.context("provide VALUE or --from-stdin")?
            };
            let secret: SecretValue = client.put(
                "/v2/secrets/write",
                &serde_json::json!({ "project": project, "key": key, "value": val }),
            )?;
            eprintln!("saved {} (v{})", secret.key_name, secret.version);
            Ok(())
        }
        SecretsCommands::Delete { key, project } => {
            client.post_no_content(
                "/v2/secrets/delete",
                &serde_json::json!({ "project": project, "key": key }),
            )?;
            eprintln!("deleted {key}");
            Ok(())
        }
    }
}
