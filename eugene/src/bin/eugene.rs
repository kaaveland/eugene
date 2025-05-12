use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use clap_complete::Shell::{Bash, Elvish, Fish, PowerShell, Zsh};
use eugene::git::{GitFilter, GitMode};
use eugene::output::output_format::GenericHint;
use eugene::output::{DetailedLockMode, LockModesWrapper, TerseLockMode};
use eugene::pg_types::lock_modes;
use eugene::pgpass::read_pgpass_file;
use eugene::script_discovery::{script_filters, SortMode};
use eugene::tempserver::TempServer;
use eugene::{
    output, parse_placeholders, perform_trace, read_script, script_discovery, ClientSource,
    WithClient,
};
use itertools::Itertools;
use postgres::Client;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "eugene")]
#[command(about = "Careful with That Lock, Eugene")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "eugene is a tool for writing safer schema changes for PostgreSQL

eugene can run your migration scripts and detect which locks that is taken by each
individual SQL statement and summarize which operations that conflict with those
locks, in other words what the script must wait for and what
concurrent transactions that would be blocked.
"
)]
struct Eugene {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
struct TraceAndLintOptions {
    /// Path to SQL migration scripts, directories, or '-' to read from stdin
    #[arg(name = "paths")]
    paths: Vec<String>,
    /// Provide name=value for replacing ${name} with value in the SQL script
    ///
    /// Can be used multiple times to provide more placeholders.
    #[arg(short = 'v', long = "var")]
    placeholders: Vec<String>,
    /// Ignore the hints with these IDs, use `eugene hints` to see available hints
    ///
    /// Can be used multiple times: `-i E3 -i E4`
    ///
    /// Or comment your SQL statement like this:
    ///
    /// `-- eugene ignore E3, E4`
    ///
    /// alter table foo add column bar json;
    ///
    /// This will ignore hints E3 and E4 for this statement only.
    #[arg(short = 'i', long = "ignore")]
    ignored_hints: Vec<String>,
    /// Output format, plain, json or markdown
    #[arg(short = 'f', long = "format", default_value = "plain", value_parser=clap::builder::PossibleValuesParser::new(["json", "markdown", "md", "plain"]))]
    format: String,
    /// Exit successfully even if problems are detected.
    ///
    /// Will still fail for syntax errors in the SQL script.
    #[arg(short = 'a', long = "accept-failures", default_value_t = false)]
    accept_failures: bool,

    /// Sort mode for script discovery, auto, name or none
    ///
    /// This is used to order scripts when a path is a directory, or many paths are provided.
    ///
    /// `auto` will sort by versions or sequence numbers.
    ///
    /// `auto` requires all files to have the same naming scheme, either flyway-style or leading sequence numbers.
    ///
    /// `name` will sort lexically by name.
    #[arg(long = "sort-mode", default_value = "auto", value_parser=clap::builder::PossibleValuesParser::new(["auto", "name", "none"]))]
    sort_mode: String,
    /// Skip the summary section for markdown output
    #[arg(short = 's', long = "skip-summary", default_value_t = false)]
    skip_summary: bool,
    /// Filter out discovered scripts that have not been changed since this git ref
    ///
    /// Pass a git ref, like a commit hash, tag, or branch name.
    #[arg(short = 'g', long = "git-diff")]
    git_diff: Option<String>,

    /// Skip SQL statements matching this regex (do not execute or lint them)
    ///
    /// For example:
    ///
    /// eugene trace --skip '.*flyway.*' --skip '.*moreToSkip.*'
    ///
    /// See https://docs.rs/regex/latest/regex/#syntax
    #[arg(long = "skip", default_value = None)]
    skip: Vec<String>,
}

impl TraceAndLintOptions {
    fn placeholders(&self) -> eugene::Result<HashMap<&str, &str>> {
        parse_placeholders(&self.placeholders)
    }
    fn format(&self) -> Result<TraceFormat> {
        self.format.as_str().try_into()
    }
    fn ignored_hints(&self) -> Vec<&str> {
        self.ignored_hints.iter().map(|s| s.as_str()).collect_vec()
    }
    fn sort_mode(&self) -> eugene::Result<SortMode> {
        self.sort_mode.as_str().try_into()
    }
    fn git_filter(&self) -> eugene::Result<GitFilter> {
        let mode: GitMode = self.git_diff.clone().into();
        let mut filter = GitFilter::empty(mode.clone());
        for path in self.paths.iter() {
            filter.extend(GitFilter::new(path, mode.clone())?)
        }
        Ok(filter)
    }
}

#[derive(Parser)]
struct ProvidedConnectionSettings {
    /// Username to use for connecting to postgres
    #[arg(short = 'U', long = "user", default_value = "postgres")]
    user: String,
    /// Database to connect to.
    #[arg(short = 'd', long = "database", default_value = "postgres")]
    database: String,
    /// Host to connect to.
    #[arg(short = 'H', long = "host", default_value = "localhost")]
    host: String,
    /// Port to connect to.
    #[arg(short = 'p', long = "port", default_value = "5432")]
    port: u16,
}

#[derive(Parser)]
struct Trace {
    #[command(flatten)]
    opts: TraceAndLintOptions,
    /// Disable creation of temporary postgres server for tracing
    ///
    /// By default, trace will create a postgres server in a temporary directory
    ///
    /// This relies on having `initdb` and `pg_ctl` in PATH, which eugene images have.
    ///
    /// Eugene deletes the temporary database cluster when done tracing.
    #[arg(long = "disable-temporary", default_value_t = false)]
    disable_temp_postgres: bool,
    /// Portgres options to pass to the temporary postgres server
    ///
    /// Example: `eugene trace -o "-c fsync=off -c log_statement=all"`
    #[arg(short = 'o', long = "postgres-options", default_value = "")]
    postgres_options: String,

    /// Initdb options to pass when creating the temporary postgres server
    ///
    /// Example: `eugene trace --initdb "--encoding=UTF8"`
    ///
    /// Supply it more than once to add multiple options.
    #[arg(long = "initdb")]
    initdb_options: Vec<String>,
    #[command(flatten)]
    connection_settings: ProvidedConnectionSettings,
    /// Commit at the end of the transaction.
    ///
    /// Commit is always enabled for the temporary server, otherwise rollback is default.
    #[arg(short = 'c', long = "commit", default_value_t = false)]
    commit: bool,
    /// Show locks that are normally not in conflict with application code.
    #[arg(short = 'e', long = "extra", default_value_t = false)]
    extra: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint SQL migration script by analyzing syntax tree
    ///
    /// `eugene lint` exits with failure if any lint is detected.
    Lint {
        #[command(flatten)]
        opts: TraceAndLintOptions,
    },
    /// Trace effects by running statements from SQL migration script
    ///
    /// `eugene trace` will set up a temporary postgres server for tracing, unless disabled.
    ///
    /// Reads $PGPASS for password to postgres, if ~/.pgpass is not found.
    ///
    /// `eugene trace` exits with failure if any problems are detected.
    Trace(Trace),
    /// List postgres lock modes
    Modes {
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// Explain what operations a lock mode allows and conflicts with
    Explain {
        /// Lock mode to explain
        mode: String,
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// Show migration hints that eugene can detect in traces
    Hints {
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },

    /// Generate shell completions for eugene
    ///
    /// Add the output to your shell configuration file or the preferred location
    /// for completions.
    Completions {
        #[arg(short, long, default_value = "bash", value_parser=clap::builder::PossibleValuesParser::new(["bash", "zsh", "fish", "pwsh", "powershell"]))]
        shell: String,
    },
}

impl TryFrom<&ProvidedConnectionSettings> for ClientSource {
    type Error = anyhow::Error;

    fn try_from(value: &ProvidedConnectionSettings) -> Result<Self, Self::Error> {
        let password = if let Ok(password) = std::env::var("PGPASS") {
            password
        } else {
            read_pgpass_file()?
                .find_password(&value.host, value.port, &value.database, &value.user)?
                .to_string()
        };
        Ok(ClientSource::new(
            value.user.clone(),
            value.database.clone(),
            value.host.clone(),
            value.port,
            password,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TraceFormat {
    Json,
    Plain,
    Markdown,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HintContainer {
    hints: Vec<GenericHint>,
}

impl TryFrom<&str> for TraceFormat {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "json" => Ok(TraceFormat::Json),
            "plain" => Ok(TraceFormat::Plain),
            "md" | "markdown" => Ok(TraceFormat::Markdown),
            _ => Err(anyhow!(
                "Invalid trace format: {}, possible choices: {:?}",
                value,
                &["json", "plain", "markdown"]
            )),
        }
    }
}

enum GetClient {
    TempDb(TempServer),
    Connect(ClientSource),
}

impl TryFrom<&Trace> for GetClient {
    type Error = anyhow::Error;

    fn try_from(value: &Trace) -> std::result::Result<Self, Self::Error> {
        if value.disable_temp_postgres {
            Ok(GetClient::Connect((&value.connection_settings).try_into()?))
        } else {
            Ok(GetClient::TempDb(TempServer::new(
                &value.postgres_options,
                &value.initdb_options,
            )?))
        }
    }
}

impl WithClient for GetClient {
    fn with_client<T>(
        &mut self,
        f: impl FnOnce(&mut Client) -> eugene::Result<T>,
    ) -> eugene::Result<T> {
        match self {
            GetClient::TempDb(temp) => temp.with_client(f),
            GetClient::Connect(settings) => settings.with_client(f),
        }
    }
}

pub fn main() -> Result<()> {
    env_logger::init();
    let args = Eugene::parse();
    match args.command {
        Some(Commands::Lint { opts }) => {
            let placeholders = opts.placeholders()?;
            let format: TraceFormat = opts.format()?;
            let mut failed = false;
            let skip = opts
                .skip
                .iter()
                .map(|s| Ok(Regex::new(s.as_str())?))
                .collect::<Result<Vec<_>>>()?;
            let filter = opts.git_filter()?;
            for read_from in script_discovery::discover_all(
                &opts.paths,
                script_filters::never,
                opts.sort_mode()?,
            )?
            .into_iter()
            .filter(|r| filter.allows(r.name()))
            {
                let script = read_script(&read_from, &placeholders)?;
                let report = eugene::lints::lint(
                    Some(script.name.clone()),
                    script.sql,
                    &opts.ignored_hints(),
                    opts.skip_summary,
                    &skip,
                )
                .map_err(|err| anyhow!("Error checking {}: {err}", script.name.as_str()))?;
                failed = failed
                    || report
                        .statements
                        .iter()
                        .any(|stmt| !stmt.triggered_rules.is_empty());
                let out = match format {
                    TraceFormat::Json => Ok(serde_json::to_string_pretty(&report)?),
                    TraceFormat::Plain => output::templates::lint_text(&report),
                    TraceFormat::Markdown => output::templates::lint_report_to_markdown(&report),
                }?;
                if !out.trim().is_empty() {
                    println!("{}", out);
                }
            }

            if failed && !opts.accept_failures {
                Err(anyhow!("Some checks failed"))
            } else {
                Ok(())
            }
        }
        Some(Commands::Trace(trace_opts)) => {
            let commit = trace_opts.commit || !trace_opts.disable_temp_postgres;
            let format = trace_opts.opts.format()?;
            let mut client_source: GetClient = (&trace_opts).try_into()?;

            let mut failed = false;
            let skip = trace_opts
                .opts
                .skip
                .iter()
                .map(|s| Ok(Regex::new(s.as_str())?))
                .collect::<Result<Vec<_>>>()?;
            let placeholders = trace_opts.opts.placeholders()?;

            let script_source = script_discovery::discover_all(
                &trace_opts.opts.paths,
                script_filters::skip_downgrade_and_repeatable,
                trace_opts.opts.sort_mode()?,
            )?;
            if !commit && script_source.len() > 1 {
                return Err(anyhow!(
                    "{} scripts detected, use --commit if you want to trace them in sequence",
                    script_source.len()
                ));
            }
            let last_script = script_source.len() - 1;
            let ignored = trace_opts.opts.ignored_hints();
            let filter = trace_opts.opts.git_filter()?;
            for (ix, read_from) in script_source.into_iter().enumerate() {
                let script = read_script(&read_from, &placeholders)?;
                let name = script.name.as_str();
                let trace = perform_trace(
                    &script,
                    &mut client_source,
                    &ignored,
                    commit,
                    &skip,
                    ix == last_script,
                )
                .map_err(|e| anyhow!("Error tracing {name}: {e}"))?;
                if filter.allows(name) {
                    let full_trace = output::full_trace_data(
                        &trace,
                        output::Settings::new(!trace_opts.extra, trace_opts.opts.skip_summary),
                    );
                    failed = failed || !trace.success();
                    let report = match format {
                        TraceFormat::Json => full_trace.to_pretty_json(),
                        TraceFormat::Plain => full_trace.to_plain_text(),
                        TraceFormat::Markdown => full_trace.to_markdown(),
                    }?;
                    if !report.trim().is_empty() {
                        println!("{}", report);
                    }
                }
            }

            if failed && !trace_opts.opts.accept_failures {
                Err(anyhow!("Some checks failed"))
            } else {
                Ok(())
            }
        }
        Some(Commands::Modes { .. }) | None => {
            let lock_modes: Vec<TerseLockMode> =
                lock_modes::LOCK_MODES.iter().map(|m| m.into()).collect();
            let wrapper = LockModesWrapper::new(lock_modes);
            println!("{}", serde_json::to_string_pretty(&wrapper)?);
            Ok(())
        }
        Some(Commands::Explain { mode, .. }) => {
            let choice = lock_modes::LOCK_MODES
                .iter()
                .find(|m| m.to_db_str() == mode || m.to_db_str().replace("Lock", "") == mode)
                .context(format!("Invalid lock mode {mode}"))?;
            let choice: DetailedLockMode = choice.into();
            println!("{}", serde_json::to_string_pretty(&choice)?);
            Ok(())
        }
        Some(Commands::Hints { .. }) => {
            let hints: Vec<_> = eugene::hint_data::ALL
                .iter()
                .copied()
                .map(GenericHint::from)
                .collect();
            let hints = HintContainer { hints };
            println!("{}", serde_json::to_string_pretty(&hints)?);
            Ok(())
        }
        Some(Commands::Completions { shell }) => {
            let sh = match shell.as_str() {
                "bash" => Ok(Bash),
                "zsh" => Ok(Zsh),
                "fish" => Ok(Fish),
                "pwsh" | "powershell" => Ok(PowerShell),
                "elvish" => Ok(Elvish),
                _ => Err(anyhow!("Unsupported shell: {shell}")),
            }?;
            let mut com = Eugene::command();
            generate(sh, &mut com, "eugene", &mut std::io::stdout());
            Ok(())
        }
    }
}
