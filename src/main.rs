//! Agent-first Redmine CLI with markdown-first output.

mod cache;
mod cli;
mod client;
mod config;
mod error;
mod mcp;
mod models;
mod output;

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cli::{Cli, Command};
use config::{load_config, ConfigPaths};
use error::AppError;
use output::{Format, Meta, OutputFormat};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up tracing
    if cli.debug {
        tracing_subscriber::registry()
            .with(fmt::layer().with_target(false).with_writer(std::io::stderr))
            .with(EnvFilter::new("debug"))
            .init();
    }

    let result = run(cli).await;
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}", e);
            e.exit_code().into()
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode, AppError> {
    let paths = ConfigPaths::new()?;
    let format = cli.format;

    if let Command::Mcp = &cli.command {
        mcp::serve(cli.url.clone(), cli.api_key.clone())
            .await
            .map_err(|e| AppError::network(format!("MCP server I/O error: {e}")))?;
        return Ok(ExitCode::SUCCESS);
    }

    // Handle commands that don't need config first
    if let Command::Profile(cmd) = &cli.command {
        return handle_profile_command(cmd, &paths, format).await;
    }

    // Load config for commands that need it
    let config = match load_config(cli.url.as_deref(), cli.api_key.as_deref(), &paths) {
        Ok(c) => c,
        Err(e) => {
            print_error(&e, format);
            return Ok(e.exit_code().into());
        }
    };

    // Handle config show (needs config but not client)
    if let Command::Config(_) = &cli.command {
        let info = cli::profile::show_config(&config);
        println!("{}", format.format_success(info, Meta::default()));
        return Ok(ExitCode::SUCCESS);
    }

    // Create client
    let client = client::RedmineClient::new(&config, cli.dry_run)?;

    // Execute command
    let result = execute_command(&cli.command, &client, &paths, format).await;

    match result {
        Ok(output) => {
            println!("{}", output);
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            print_error(&e, format);
            Ok(e.exit_code().into())
        }
    }
}

async fn handle_profile_command(
    cmd: &cli::profile::ProfileCommand,
    paths: &ConfigPaths,
    format: OutputFormat,
) -> Result<ExitCode, AppError> {
    use cli::profile::ProfileCommand;

    let result = match cmd {
        ProfileCommand::Add(args) => cli::profile::add_profile(args, paths)
            .map(|r| format.format_success(r, Meta::default())),
        ProfileCommand::Use(args) => cli::profile::use_profile(args, paths)
            .map(|r| format.format_success(r, Meta::default())),
        ProfileCommand::List => {
            cli::profile::list_profiles(paths).map(|r| format.format_success(r, Meta::default()))
        }
        ProfileCommand::Delete(args) => cli::profile::delete_profile(args, paths)
            .map(|r| format.format_success(r, Meta::default())),
    };

    match result {
        Ok(output) => {
            println!("{}", output);
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            print_error(&e, format);
            Ok(e.exit_code().into())
        }
    }
}

pub(crate) async fn execute_command(
    command: &Command,
    client: &client::RedmineClient,
    paths: &ConfigPaths,
    format: OutputFormat,
) -> Result<String, AppError> {
    match command {
        Command::Ping => {
            let result = cli::ping::execute(client).await?;
            Ok(format.format_success(result, Meta::default()))
        }

        Command::Me => {
            let user = client.me().await?;
            Ok(format.format_success(user, Meta::default()))
        }

        Command::Profile(_) | Command::Config(_) | Command::Mcp => {
            // Already handled
            unreachable!()
        }

        Command::Project(cmd) => {
            use cli::project::ProjectCommand;
            match cmd {
                ProjectCommand::List(args) => {
                    let result = cli::project::list(client, args).await?;
                    let meta = Meta::paginated(
                        result.total_count.unwrap_or(0),
                        result.limit.unwrap_or(25),
                        result.offset.unwrap_or(0),
                    );
                    Ok(format.format_success(result, meta))
                }
                ProjectCommand::Get(args) => {
                    let result = cli::project::get(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
            }
        }

        Command::Issue(cmd) => {
            use cli::issue::{AttachmentCommand, IssueCommand, RelationCommand};
            match cmd {
                IssueCommand::List(args) => {
                    let result = cli::issue::list(client, args).await?;
                    let meta = Meta::paginated(
                        result.total_count.unwrap_or(0),
                        result.limit.unwrap_or(25),
                        result.offset.unwrap_or(0),
                    );
                    Ok(format.format_success(result, meta))
                }
                IssueCommand::Get(args) => {
                    let result = cli::issue::get(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                IssueCommand::Create(args) => {
                    let result = cli::issue::create(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                IssueCommand::Update(args) => {
                    let result = cli::issue::update(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                IssueCommand::Attachment(cmd) => match cmd {
                    AttachmentCommand::List(args) => {
                        let result = cli::issue::attachment_list(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                    AttachmentCommand::Download(args) => {
                        let result = cli::issue::attachment_download(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                    AttachmentCommand::Upload(args) => {
                        let result = cli::issue::attachment_upload(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                },
                IssueCommand::Relation(cmd) => match cmd {
                    RelationCommand::List(args) => {
                        let result = cli::issue::relation_list(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                    RelationCommand::Add(args) => {
                        let result = cli::issue::relation_add(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                    RelationCommand::Remove(args) => {
                        let result = cli::issue::relation_remove(client, args).await?;
                        Ok(format.format_success(result, Meta::default()))
                    }
                },
            }
        }

        Command::Query(cmd) => {
            use cli::query::QueryCommand;
            match cmd {
                QueryCommand::List(args) => {
                    let result = cli::query::list(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
            }
        }

        Command::Version(cmd) => {
            use cli::version::VersionCommand;
            match cmd {
                VersionCommand::List(args) => {
                    let result = cli::version::list(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
            }
        }

        Command::Time(cmd) => {
            use cli::time::TimeCommand;
            match cmd {
                TimeCommand::Activities(sub) => {
                    use cli::time::ActivitiesCommand;
                    match sub {
                        ActivitiesCommand::List(args) => {
                            let result = cli::time::list_activities(client, paths, args).await?;
                            Ok(format.format_success(result, Meta::default()))
                        }
                    }
                }
                TimeCommand::Create(args) => {
                    let result = cli::time::create(client, paths, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                TimeCommand::List(args) => {
                    let result = cli::time::list(client, args).await?;
                    let meta = result.meta();
                    Ok(format.format_success(result, meta))
                }
                TimeCommand::Get(args) => {
                    let result = cli::time::get(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                TimeCommand::Update(args) => {
                    let result = cli::time::update(client, paths, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
                TimeCommand::Delete(args) => {
                    let result = cli::time::delete(client, args).await?;
                    Ok(format.format_success(result, Meta::default()))
                }
            }
        }

        Command::User(cmd) => {
            use cli::user::UserCommand;
            match cmd {
                UserCommand::List(args) => {
                    let result = cli::user::list(client, args).await?;
                    let meta = Meta::paginated(
                        result.total_count.unwrap_or(0),
                        result.limit.unwrap_or(25),
                        result.offset.unwrap_or(0),
                    );
                    Ok(format.format_success(result, meta))
                }
                UserCommand::Me => {
                    let user = client.me().await?;
                    Ok(format.format_success(user, Meta::default()))
                }
            }
        }
    }
}

fn print_error(error: &AppError, format: OutputFormat) {
    let output = format.format_error(error);
    eprintln!("{}", output);
}
