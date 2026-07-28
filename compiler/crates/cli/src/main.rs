use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};
use miette::Report;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vinyl_codegen::CodegenBackend;

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
    Run { file: Option<PathBuf> },
    /// Parse and type check a Vinyl file without generating code
    Check { file: Option<PathBuf> },
    /// Parse, type check, and AOT compile a Vinyl file to a native binary
    Build {
        file: Option<PathBuf>,
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
    },
    /// Format Vinyl source files
    #[command(alias = "format")]
    Fmt { path: Option<PathBuf> },
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

fn compile_and_report(file: &std::path::Path) -> eyre::Result<Vec<vinyl_typecheck::hir::HirItem>> {
    let (compiled, warnings) =
        vinyl_compiler::compile_entry(file, None).map_err(|errors| {
            for error in errors {
                eprintln!("{:?}", Report::from(error));
            }
            eyre::eyre!("compilation failed")
        })?;
    for w in warnings {
        eprintln!("{:?}", Report::from(w));
    }
    Ok(compiled.items)
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
            let file = file.unwrap_or_else(|| PathBuf::from("src"));
            let _items = compile_and_report(&file)?;
        }
        Command::Run { file } => {
            let file = file.unwrap_or_else(|| PathBuf::from("src"));
            let items = compile_and_report(&file)?;
            jit_and_run(&items)?;
        }
        Command::Build { file, output } => {
            let file = file.unwrap_or_else(|| PathBuf::from("src"));
            let items = compile_and_report(&file)?;
            println!(
                "compiled {} -> {} ({} items, codegen not yet implemented)",
                file.display(),
                output.display(),
                items.len()
            );
        }
        Command::Fmt { path } => {
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            if path.is_file() {
                vinyl_formatter::format_path(&path).map_err(|errors| {
                    for e in &errors {
                        eprintln!("{e}");
                    }
                    eyre::eyre!("formatting failed")
                })?;
            } else {
                vinyl_formatter::format_project(&path).map_err(|errors| {
                    for e in &errors {
                        eprintln!("{e}");
                    }
                    eyre::eyre!("formatting failed")
                })?;
            }
        }
    }
    Ok(())
}
