use std::path::PathBuf;
use std::sync::OnceLock;

use clap::{ArgAction, Parser, Subcommand};
use miette::Report;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use vinyl_codegen::CodegenBackend;
use vinyl_compiler::CompileError;

static FILTER_RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

#[derive(Parser)]
#[command(name = "vinyl", version, about = "Vinyl language compiler")]
struct Cli {
    /// Increase verbosity (-v for DEBUG, -vv for TRACE, -vvv for global TRACE)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse, type check, and JIT compile a Vinyl file
    Run { file: PathBuf },
    /// Parse and type check a Vinyl file without generating code
    Check { file: PathBuf },
    /// Parse, type check, and AOT compile a Vinyl file to a native binary
    Build {
        file: PathBuf,
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
    },
}

fn init_tracing() -> eyre::Result<()> {
    let env_filter = EnvFilter::new("info");
    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    FILTER_RELOAD_HANDLE
        .set(reload_handle)
        .map_err(|_| eyre::eyre!("Tracing reload handle already initialised"))?;

    Ok(())
}

fn setup_tracing(verbose: u8) -> eyre::Result<()> {
    let env_filter = if verbose > 0 {
        let crate_name = env!("CARGO_CRATE_NAME");
        match verbose {
            1 => EnvFilter::new(format!("{crate_name}=debug")),
            2 => EnvFilter::new(format!("{crate_name}=trace")),
            _ => EnvFilter::new("trace"),
        }
    } else {
        match EnvFilter::try_from_default_env() {
            Ok(filter) => filter,
            Err(_) => return Ok(()),
        }
    };

    let handle = FILTER_RELOAD_HANDLE
        .get()
        .ok_or_else(|| eyre::eyre!("Tracing not initialised — call init_tracing first"))?;

    handle.reload(env_filter)?;

    Ok(())
}

fn compile_and_report(
    source: &str,
    source_name: &str,
) -> eyre::Result<Vec<vinyl_typecheck::hir::HirItem>> {
    match vinyl_compiler::compile(source, source_name) {
        Ok(items) => Ok(items),
        Err(errors) => {
            for error in errors {
                match error {
                    CompileError::Parse(e) => eprintln!("{:?}", Report::from(e)),
                    CompileError::Lower(e) => eprintln!("{:?}", Report::from(e)),
                    CompileError::TypeError(e) => eprintln!("{:?}", Report::from(e)),
                }
            }
            std::process::exit(1);
        }
    }
}

fn jit_and_run(items: &[vinyl_typecheck::hir::HirItem]) -> eyre::Result<()> {
    let mut backend =
        vinyl_codegen::CraneliftBackend::new().map_err(|e| eyre::eyre!("jit init: {e}"))?;
    backend
        .compile(items)
        .map_err(|e| eyre::eyre!("jit compile: {e}"))?;
    let result = backend.run().map_err(|e| eyre::eyre!("jit run: {e}"))?;
    println!("{}", result);
    Ok(())
}

fn main() -> eyre::Result<()> {
    init_tracing()?;

    let cli = Cli::parse();
    setup_tracing(cli.verbose)?;

    match cli.command {
        Command::Check { file } => {
            let source_name = file.to_string_lossy();
            let source = std::fs::read_to_string(&file)?;
            let _items = compile_and_report(&source, &source_name)?;
        }
        Command::Run { file } => {
            let source_name = file.to_string_lossy();
            let source = std::fs::read_to_string(&file)?;
            let items = compile_and_report(&source, &source_name)?;
            jit_and_run(&items)?;
        }
        Command::Build { file, output } => {
            let source_name = file.to_string_lossy();
            let source = std::fs::read_to_string(&file)?;
            let items = compile_and_report(&source, &source_name)?;
            println!(
                "compiled {} -> {} ({} items, codegen not yet implemented)",
                file.display(),
                output.display(),
                items.len()
            );
        }
    }
    Ok(())
}
