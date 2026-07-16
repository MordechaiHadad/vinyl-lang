use clap::{Parser, Subcommand};
use miette::Report;
use std::path::PathBuf;
use vinyl_compiler::CompileError;

#[derive(Parser)]
#[command(name = "vinyl", version, about = "Vinyl language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and JIT compile a Vinyl file
    Run {
        file: PathBuf,
    },
    /// Parse and AOT compile a Vinyl file to a native binary
    Build {
        file: PathBuf,
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
    },
}

fn compile_and_report(source: &str) -> eyre::Result<Vec<vinyl_parser::ast::Item>> {
    match vinyl_compiler::compile(source) {
        Ok(items) => Ok(items),
        Err(errors) => {
            for error in errors {
                match error {
                    CompileError::Parse(e) => eprintln!("{:?}", Report::from(e)),
                    CompileError::Lower(e) => eprintln!("{:?}", Report::from(e)),
                }
            }
            std::process::exit(1);
        }
    }
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Command::Run { file } => {
            let source = std::fs::read_to_string(&file)?;
            let items = compile_and_report(&source)?;
            println!("parsed and lowered {} items:", items.len());
            for item in &items {
                println!("  {item:?}");
            }
        }
        Command::Build { file, output } => {
            let source = std::fs::read_to_string(&file)?;
            let items = compile_and_report(&source)?;
            println!("compiled {} -> {} ({} items, codegen not yet implemented)", file.display(), output.display(), items.len());
        }
    }
    Ok(())
}
