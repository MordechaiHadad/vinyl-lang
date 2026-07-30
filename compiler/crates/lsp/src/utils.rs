use clap::{ArgAction, Parser};
use eyre::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "vinyl-lsp", version, about = "Vinyl language server")]
pub struct Cli {
    /// Increase verbosity (-v for DEBUG, -vv for TRACE, -vvv for global TRACE)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,
}

pub fn init_tracing(verbose: u8) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stderr());

    let filter = if verbose > 0 {
        let crate_name = env!("CARGO_CRATE_NAME");
        match verbose {
            1 => tracing_subscriber::EnvFilter::new(format!("{crate_name}=debug")),
            2 => tracing_subscriber::EnvFilter::new(format!("{crate_name}=trace")),
            _ => tracing_subscriber::EnvFilter::new("trace"),
        }
    } else if is_tty {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .without_time()
                .with_target(false)
                .with_ansi(is_tty)
                .with_writer(std::io::stderr),
        )
        .try_init()?;

    Ok(())
}
