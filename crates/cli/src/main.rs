use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};
use miette::Report;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vinyl_codegen::CodegenBackend;
use vinyl_compiler::CompileError;

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

fn init_tracing(verbose: u8) -> eyre::Result<()> {
    let env_filter = if verbose > 0 {
        let crate_name = env!("CARGO_CRATE_NAME");
        match verbose {
            1 => EnvFilter::new(format!("{crate_name}=debug")),
            2 => EnvFilter::new(format!("{crate_name}=trace")),
            _ => EnvFilter::new("trace"),
        }
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

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
    let has_main = items.iter().any(|item| {
        matches!(
            &item.kind,
            vinyl_typecheck::hir::HirItemKind::Function(f) if f.name == "main"
        )
    });
    if !has_main {
        warn!("no main function found");
    }

    let mut backend =
        vinyl_codegen::CraneliftBackend::new().map_err(|e| eyre::eyre!("jit init: {e}"))?;
    backend
        .compile(items)
        .map_err(|e| eyre::eyre!("jit compile: {e}"))?;
    let result = backend.run().map_err(|e| eyre::eyre!("jit run: {e}"))?;
    if has_main {
        println!("{}", result);
    }
    Ok(())
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;

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
