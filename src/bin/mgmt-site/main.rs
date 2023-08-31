// mgmt-site sets up a directory containing repos and configuration values for a DE deployment site.
//
// A site consists of one or more DE deployments.

use anyhow::Context;
use clap::{arg, ArgAction, Command};
use mgmt::config_values::config;
use mgmt::db;
use mgmt::dolt;
use mgmt::git;
use mgmt::ops;
use std::path::{Path, PathBuf};

use sqlx::mysql::MySqlPoolOptions;

/**
 * Set up the CLI for the mgmt-site binary.
 */
fn cli() -> Command {
    Command::new("mgmt-site")
        .about(
            "Sets up directory containing repos and configuration values for a DE deployment site.",
        )
        .args_conflicts_with_subcommands(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("init").args([
                arg!(-d --dir [DIR] "Directory to initialize")
                    .help("The directory containing the site information. Defaults to the currect directory.")
                    .default_value(".")
                    .value_parser(clap::value_parser!(String)),
                arg!(-r --"db-repo" [DB_REPO] "The Dolt DB repo to set up and use for initializing the local DB.")
                    .required(true)
                    .value_parser(clap::value_parser!(String)),
                arg!(-n --"db-name" [DB_NAME] "The name of the DB")
                    .default_value("de_releases")
                    .value_parser(clap::value_parser!(String)),
                arg!(-C --"no-db-clone" "Do not clone the Dolt DB repo")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(-R --"no-repo-clone" "Do not clone the repos")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(-f --force "Overwrite existing files")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(-E --"no-env" "Do not prompt the user for values for an environment")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(-D --"no-defaults" "Do not write out the default values to a file in the site directory")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(-V --"no-values" "Do not write out the config values for the environment to a file in the site directory")
                    .action(ArgAction::SetTrue)
                    .value_parser(clap::value_parser!(bool)),
                arg!(--"defaults-filename" [DEFAULTS_FILENAME] "The name of the file to write the default values to in the site directory")
                    .default_value("defaults.yaml")
                    .value_parser(clap::value_parser!(String)),
                arg!(--"values-filename" [VALUES_FILENAME] "The name of the file to write the config values to in the site directory")
                    .default_value("deployment.yaml")
                    .value_parser(clap::value_parser!(String)),
            ]),
        )
        .subcommand(
            Command::new("deploy")
                .args([
                    arg!(-d --dir [DIR] "Directory to deploy from")
                        .default_value(".")
                        .value_parser(clap::value_parser!(PathBuf)),
                    arg!(-e --env [ENV] "The environment to deploy")
                        .required(true)
                        .value_parser(clap::value_parser!(String)),
                    arg!(-s --service [SERVICE] "The service to deploy")
                        .required(false)
                        .action(ArgAction::Append)
                        .value_parser(clap::value_parser!(String)),
                    arg!(--"defaults-filename" [DEFAULTS_FILENAME] "The file containing the default configuration values")
                        .default_value("defaults.yaml")
                        .value_parser(clap::value_parser!(PathBuf)),
                    arg!(--"values-filename" [VALUES_FILENAME] "The file containing the configuration values for the environment")
                        .default_value("deployment.yaml")
                        .value_parser(clap::value_parser!(PathBuf)),
                ])
        )
}

#[derive(Debug, Clone, PartialEq)]
struct InitOpts {
    dir: String,
    db_repo: String,
    db_name: String,
    force: bool,
    no_db_clone: bool,
    no_repo_clone: bool,
    no_env: bool,
    no_defaults: bool,
    no_values: bool,
    defaults_filename: String,
    values_filename: String,
}

// Create the site directory if it doesn't already exist.
// If it does exist, and force is true, delete it and recreate it.
fn create_site_dir(opts: &InitOpts) -> anyhow::Result<()> {
    let dir = &opts.dir;
    let force = opts.force;
    let site_exists = std::path::Path::new(dir).exists();
    if site_exists && force {
        std::fs::remove_dir_all(dir)?;
    } else if site_exists {
        return Err(anyhow::anyhow!(
            "Directory {} already exists. Use -f or --force to overwrite.",
            dir
        ));
    } else {
        let repo_dir = Path::new(dir).join("repos");
        std::fs::create_dir_all(repo_dir)?;
    }
    Ok(())
}

// Create the dolt database directory inside of the site directory.
// If force is true, delete the directory and recreate it.
fn create_db_dir(opts: &InitOpts) -> anyhow::Result<PathBuf> {
    let dir = &opts.dir;
    let db_name = &opts.db_name;
    let force = opts.force;
    let db_dir = Path::new(dir).join(db_name);
    if db_dir.exists() && force {
        std::fs::remove_dir_all(&db_dir)?;
    } else if db_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory {} already exists. Use -f or --force to overwrite.",
            db_dir.to_str().unwrap()
        ));
    }
    std::fs::create_dir_all(&db_dir)?;
    Ok(db_dir)
}

// Use the dolt command to clone the initial database state from the remote.
fn clone_db(opts: &InitOpts) -> anyhow::Result<PathBuf> {
    let db_repo = &opts.db_repo;
    let db_dir = create_db_dir(&opts)?;
    let db_dir_str = db_dir
        .to_str()
        .context("could not get name of the database directory")?;
    dolt::clone(db_repo, db_dir_str)?;
    Ok(db_dir)
}

async fn init(opts: &InitOpts) -> anyhow::Result<()> {
    // Create the site directory.
    create_site_dir(&opts)?;

    let db_dir: PathBuf;

    println!("Cloning the database from {}...", &opts.db_repo);
    // Clone the base database.
    if !opts.no_db_clone {
        db_dir = clone_db(&opts)?;
    } else {
        db_dir = PathBuf::from(&opts.dir).join(&opts.db_name);
    }
    println!("Done cloning the database.\n");

    println!("Starting the database...");
    // Start the database
    let db_dir_str = db_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("failed to get database directory as string"))?;
    let db_handle = dolt::start(db_dir_str)?;
    println!("Done staring the database.\n");

    println!("Connecting to the database...");
    // Connect to the database.
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&format!("mysql://root@127.0.0.1:3306/{}", &opts.db_name))
        .await?;
    let mut tx = pool.begin().await?;
    println!("Done connecting to the database.\n");

    // Get the list of repos.
    let repos = db::get_repos(&mut tx).await?;

    println!("Cloning the repos...");
    // Clone each of the repos.
    for repo in repos {
        let (repo_url, repo_name) = repo;
        let repo_dir = Path::new(&opts.dir).join("repos").join(&repo_name);
        let repo_dir_str = repo_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("failed to get repo directory as string"))
            .unwrap();

        println!("Cloning {} into {}", repo_url, repo_dir_str);
        if !opts.no_repo_clone {
            git::clone(&repo_url, repo_dir_str)?;
        } else {
            println!("Skipping cloning of {}", repo_url);
        }
        println!("");
    }
    println!("Done cloning the repos.\n");

    let mut env_config = config::ConfigValues::default();

    if !opts.no_env {
        println!("Setting up the environment...");
        env_config.ask_for_info(&mut tx).await?;
        println!("Done setting up the environment.\n");
    }

    // Write out the default config values into the site directory.
    if !opts.no_defaults {
        println!("Writing out the default values...");
        let defaults_filename = Path::new(&opts.dir).join(&opts.defaults_filename);
        ops::render_default_values(&pool, Some(defaults_filename)).await?;
        println!("Done writing out the default values.\n");
    }

    tx.commit().await?;

    if !opts.no_env && !opts.no_values {
        println!("Writing out the environment config values...");
        println!("env: {:?}", env_config.environment);
        let values_filename = Path::new(&opts.dir).join(&opts.values_filename);
        let mut section_option = config::SectionOptions::default();
        section_option.set_all(true)?;
        ops::render_values(
            &pool,
            &env_config.environment,
            &section_option,
            Some(values_filename),
        )
        .await?;
        println!("Done writing out the environment config values.\n");
    }

    // Clean up and shut down
    println!("Shutting down the database...");
    pool.close().await;
    db_handle.kill()?;
    println!("Done shutting down the database.\n");

    Ok(())
}

async fn deploy(
    env: &str,
    services: Vec<String>,
    dir: &PathBuf,
    defaults: &PathBuf,
    values: &PathBuf,
) -> anyhow::Result<()> {
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", matches)) => {
            let dir = matches.get_one::<String>("dir").ok_or_else(|| {
                anyhow::anyhow!("No directory specified. Use -d or --dir to specify a directory.")
            })?;

            let db_repo = matches.get_one::<String>("db-repo").ok_or_else(|| {
                anyhow::anyhow!("No Dolt DB remote specified. Use -r or --db-remote to specify a Dolt DB remote.")
            })?;

            let db_name = matches.get_one::<String>("db-name").ok_or_else(|| {
                anyhow::anyhow!(
                    "No Dolt DB name specified. Use -n or --db-name to specify a Dolt DB name."
                )
            })?;

            let no_db_clone = matches.get_flag("no-db-clone");
            let no_repo_clone = matches.get_flag("no-repo-clone");
            let force = matches.get_flag("force");
            let no_env = matches.get_flag("no-env");
            let no_defaults = matches.get_flag("no-defaults");
            let no_values = matches.get_flag("no-values");
            let defaults_filename = matches.get_one::<String>("defaults-filename").ok_or_else(|| {
                anyhow::anyhow!("No defaults filename specified. Use --defaults-filename to specify a defaults filename.")
            })?;
            let values_filename = matches.get_one::<String>("values-filename").ok_or_else(|| {
                anyhow::anyhow!("No values filename specified. Use --values-filename to specify a values filename.")
            })?;

            let opts = InitOpts {
                dir: dir.clone(),
                db_repo: db_repo.clone(),
                db_name: db_name.clone(),
                force,
                no_db_clone,
                no_repo_clone,
                no_env,
                no_defaults,
                no_values,
                defaults_filename: defaults_filename.clone(),
                values_filename: values_filename.clone(),
            };
            init(&opts).await?;
            println!("Site initialized in {}", dir);
        }
        Some(("deploy", matches)) => {
            let dir = matches.get_one::<PathBuf>("dir").ok_or_else(|| {
                anyhow::anyhow!("No directory specified. Use -d or --dir to specify a directory.")
            })?;

            let env = matches.get_one::<String>("env").ok_or_else(|| {
                anyhow::anyhow!(
                    "No environment specified. Use -e or --env to specify an environment."
                )
            })?;

            let services = matches
                .get_many::<String>("service")
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No services specified. Use -s or --service to specify a service."
                    )
                })?
                .map(|v| v.to_string())
                .collect::<Vec<_>>();

            let defaults_filename = matches.get_one::<PathBuf>("defaults-filename").ok_or_else(|| {
                anyhow::anyhow!("No defaults filename specified. Use --defaults-filename to specify a defaults filename.")
            })?;

            let values_filename = matches.get_one::<PathBuf>("values-filename").ok_or_else(|| {
                anyhow::anyhow!("No values filename specified. Use --values-filename to specify a values filename.")
            })?;

            deploy(&env, services, dir, defaults_filename, values_filename).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
