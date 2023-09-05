use anyhow::{anyhow, Context, Result};
use clap::{arg, ArgAction, Command};
//use mgmt::{db, dolt, git, ops};
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use std::path::{Path, PathBuf};

fn cli() -> Command {
    Command::new("mgmt-release")
        .about("Creates and posts a release to a git repository")
        .args_conflicts_with_subcommands(true)
        .subcommand_required(true)
        .arg(
            arg!(-d --"database-url" <DATABASE>)
                .help("The URL of the MySQL database to connect to.")
                .default_value("mysql:://root@127.0.0.1:3306/de_releases")
                .value_parser(clap::value_parser!(String)),
        )
        .subcommand(
            Command::new("create").args([
                arg!(-l --"local" [LOCAL] "Directory to use for the release")
                    .help("A local directory to use for staging the release")
                    .required(false)
                    .default_value("release")
                    .value_parser(clap::value_parser!(PathBuf)),
                arg!(-s --"skip" [SKIP] "A service to skip for the release")
                    .required(false)
                    .action(ArgAction::Append)
                    .value_parser(clap::value_parser!(String)),
                arg!(-e --env [ENV] "The environment to release")
                    .required(true)
                    .value_parser(clap::value_parser!(String)),
                arg!(-r --"repo" [REPO] "The repository to release to")
                    .required(true)
                    .value_parser(clap::value_parser!(String)),
            ]),
        )
        .subcommand(
            Command::new("preview")
                .about("Generates a preview of the release")
                .args([
                    arg!(-s --"skip" [SKIP] "A service to skip for the release")
                        .required(false)
                        .action(ArgAction::Append)
                        .value_parser(clap::value_parser!(String)),
                    arg!(-e --env [ENV] "The environment to release")
                        .required(true)
                        .value_parser(clap::value_parser!(String)),
                    arg!(-r --"repo" [REPO] "The repository to release to"),
                ]),
        )
}

async fn create_release(
    pool: &Pool<MySql>,
    env: &str,
    repo: &str,
    local: &Path,
    skips: Vec<String>,
) -> Result<()> {
    Ok(())
}

async fn preview_release(
    pool: &Pool<MySql>,
    env: &str,
    repo: &str,
    skips: Vec<String>,
) -> Result<()> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = cli().get_matches();

    let database_url = matches
        .get_one::<String>("database-url")
        .unwrap_or_else(|| {
            panic!("No database URL specified. Use --database-url <url> to specify a database URL.")
        });

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    match matches.subcommand() {
        Some(("create", matches)) => {
            let env = matches.get_one::<String>("env").ok_or_else(|| {
                anyhow!("No environment provided. Use --env <env> to specify an environment.")
            })?;

            let repo = matches.get_one::<String>("repo").ok_or_else(|| {
                anyhow!("No repository provided. Use --repo <repo> to specify a repository.")
            })?;

            let local = matches.get_one::<PathBuf>("local").ok_or_else(|| {
                anyhow!("No local directory provided. Use --local <local> to specify a local directory.")
            })?;

            let skips = matches
                .get_many::<String>("skip")
                .unwrap_or_default()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            let local_canon = local.canonicalize().context(format!(
                "Failed to canonicalize the local directory: {}",
                local.display()
            ))?;

            create_release(&pool, &env, &repo, &local_canon, skips).await?;
        }

        Some(("preview", matches)) => {
            let env = matches.get_one::<String>("env").ok_or_else(|| {
                anyhow!("No environment provided. Use --env <env> to specify an environment.")
            })?;

            let repo = matches.get_one::<String>("repo").ok_or_else(|| {
                anyhow!("No repository provided. Use --repo <repo> to specify a repository.")
            })?;

            let skips = matches
                .get_many::<String>("skip")
                .unwrap_or_default()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            preview_release(&pool, &env, &repo, skips).await?;
        }

        _ => {
            println!("No subcommand was used");
        }
    }

    Ok(())
}
