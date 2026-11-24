use std::process::ExitCode;

use clap::Parser;
use meta_ast::interface::args::{Cli, Command, GlobalArgs};
use meta_ast::interface::commands;
use meta_ast::interface::report::DiagnosticFormat;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_diagnostics = cli.command.diagnostics() == DiagnosticFormat::Json;
    match run(cli, json_diagnostics) {
        Ok(status) => status,
        Err(error) => {
            print_error(&error, json_diagnostics);
            ExitCode::from(exit_status(&error))
        }
    }
}

/// Usage and configuration problems exit with 2, every other failure with 1.
fn exit_status(error: &anyhow::Error) -> u8 {
    match error.downcast_ref::<meta_ast::Error>() {
        Some(meta_ast::Error::Config(_)) => 2,
        _ => 1,
    }
}

/// Report a failure on stderr: the `error:` line outside JSON mode, one JSON
/// error document in JSON mode, where the machine channel must stay clean.
fn print_error(error: &anyhow::Error, json_diagnostics: bool) {
    if !json_diagnostics {
        eprintln!("error: {error:#}");
        return;
    }
    // The tripped-policy failure is already folded into the analysis document.
    let tripped_policy = matches!(
        error.downcast_ref::<meta_ast::Error>(),
        Some(meta_ast::Error::Diagnostics { .. })
    );
    if !tripped_policy {
        meta_ast::interface::report::print_error_json(error);
    }
}

/// Parse first, so help and version stay clean, then install the subscriber
/// before anything can log or panic.
fn run(cli: Cli, json_diagnostics: bool) -> anyhow::Result<ExitCode> {
    install_subscriber(&cli.globals);

    // The banner shares stderr with the JSON document, so it must stay off
    // the machine channel when diagnostics are emitted as JSON.
    if !cli.globals.no_banner && !json_diagnostics {
        meta_ast::interface::banner::print_banner();
    }
    meta_ast::language::validate_queries();

    match cli.command {
        Command::Inspect(args) => commands::inspect(args),
        Command::Graph(args) => commands::graph(args),
        #[cfg(feature = "metacall-deploy")]
        Command::Deploy(args) => commands::deploy(args),
    }
}

/// `RUST_LOG` wins when it is set; otherwise the global flags pick the level.
fn install_subscriber(globals: &GlobalArgs) {
    use tracing_subscriber::EnvFilter;

    let filter = match std::env::var("RUST_LOG") {
        Ok(_) => EnvFilter::from_default_env(),
        Err(_) => EnvFilter::new(globals.log_level()),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
