mod cli;
mod commands;
mod human;
mod i18n;
mod maiko;
mod output;

use clap::{CommandFactory, FromArgMatches};
use envenb_core::EnvEnb;

#[tokio::main]
async fn main() {
    // `envenb ... | head` closes the pipe early; exit quietly instead of panicking
    // inside a println. Rust ignores SIGPIPE by default, so restore the default.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Open the data directory first so the persisted language setting can shape
    // the help text. Failures are deferred until a command actually needs the core.
    let core = EnvEnb::open_default().await;
    let language = match &core {
        Ok(c) => c.settings().await.ok().map(|s| s.language),
        Err(_) => None,
    };
    i18n::init(language.as_deref());

    let command = cli::localize(cli::Cli::command());
    let matches = command.clone().get_matches();
    let args = cli::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    init_tracing(args.verbose);

    // `envenb` with no subcommand: greet with the maiko (interactive only),
    // then print the usage and exit successfully.
    if args.command.is_none() {
        if maiko::should_animate(args.no_animation, args.json) {
            maiko::splash();
        }
        let mut cmd = command;
        cmd.print_help().ok();
        println!();
        return;
    }

    if let Err(err) = commands::run(args, core).await {
        eprintln!("{}: {}", i18n::tr("error", "エラー"), i18n::describe_error(&err));
        std::process::exit(1);
    }
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::EnvFilter;
    let default = if verbose { "envenb=debug" } else { "envenb=warn" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();
}
