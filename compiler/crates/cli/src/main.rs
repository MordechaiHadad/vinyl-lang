use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, Subcommand};
use miette::{Diagnostic, Report, ReportHandler, Severity};
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vinyl_codegen::CodegenBackend;

#[derive(Debug)]
struct RustcReportHandler;

impl ReportHandler for RustcReportHandler {
    fn debug(
        &self,
        diagnostic: &dyn Diagnostic,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let colors = std::io::stderr().is_terminal();
        let severity = match diagnostic.severity() {
            Some(Severity::Warning) => "warning",
            Some(Severity::Advice) => "advice",
            Some(Severity::Error) | None => "error",
        };
        let severity_color = match diagnostic.severity() {
            Some(Severity::Warning) => "33",
            Some(Severity::Advice) => "36",
            Some(Severity::Error) | None => "31",
        };

        write!(formatter, "{}", color(severity, severity_color, colors))?;
        if let Some(code) = diagnostic.code() {
            write!(formatter, "{}", color(&format!("[{code}]"), "1", colors))?;
        }
        writeln!(formatter, ": {}", diagnostic)?;

        let labels = diagnostic
            .labels()
            .map(|labels| labels.collect::<Vec<_>>())
            .unwrap_or_default();

        if let (Some(source), Some(first_label)) = (diagnostic.source_code(), labels.first())
            && let Ok(contents) = source.read_span(first_label.inner(), 1, 1)
        {
            let source_name = contents.name().unwrap_or("<source>");
            let first_line = contents.line() + 1;
            let first_column = contents.column() + 1;
            writeln!(
                formatter,
                " {}",
                color(
                    &format!("--> {source_name}:{first_line}:{first_column}"),
                    "36",
                    colors,
                )
            )?;
            writeln!(formatter, "{}", color("  |", "34", colors))?;
            render_source(formatter, contents.as_ref(), &labels, colors)?;
            writeln!(formatter, "{}", color("  |", "34", colors))?;
        }

        if let Some(help) = diagnostic.help() {
            writeln!(formatter, "  = {}: {help}", color("help", "32", colors))?;
        }
        if let Some(url) = diagnostic.url() {
            writeln!(formatter, "  = {}: {url}", color("note", "36", colors))?;
        }
        Ok(())
    }
}

fn color(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn render_source(
    formatter: &mut std::fmt::Formatter<'_>,
    contents: &dyn miette::SpanContents<'_>,
    labels: &[miette::LabeledSpan],
    colors: bool,
) -> std::fmt::Result {
    let source = String::from_utf8_lossy(contents.data());
    let source_start = contents.span().offset();
    let line_number_start = contents.line() + 1;
    let line_number_end = line_number_start + contents.line_count().saturating_sub(1);
    let line_number_width = line_number_end.to_string().len();
    let mut line_start = source_start;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let text = line.strip_suffix('\n').unwrap_or(line);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let line_number = line_number_start + line_index;
        writeln!(
            formatter,
            "{} {} {text}",
            color(&format!("{line_number:>line_number_width$}"), "34", colors),
            color("|", "34", colors),
        )?;

        let line_end = line_start + text.len();
        for label in labels {
            let label_start = label.offset();
            let label_end = label.offset() + label.len();
            if label_start > line_end || label_end < line_start {
                continue;
            }

            let marker_start = label_start.saturating_sub(line_start).min(text.len());
            let marker_end = label_end.saturating_sub(line_start).min(text.len());
            let marker_length = marker_end.saturating_sub(marker_start).max(1);
            let marker = color(&"^".repeat(marker_length), "31", colors);
            let label_text = label.label().unwrap_or("");
            writeln!(
                formatter,
                "{:>width$} {} {:>start$}{marker} {label_text}",
                "",
                color("|", "34", colors),
                "",
                width = line_number_width,
                start = marker_start + 1,
                label_text = color(label_text, "31", colors),
            )?;
        }

        line_start += line.len();
    }

    Ok(())
}

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
    let (compiled, warnings) = vinyl_compiler::compile_entry(file, None).map_err(|errors| {
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
    if let Some(function) = items.iter().find_map(|item| match &item.kind {
        vinyl_typecheck::hir::HirItemKind::Function(function) if function.name == "main" => {
            Some(function)
        }
        _ => None,
    }) && !matches!(
        function.return_type,
        vinyl_parser::ast::types::Type::Primitive(vinyl_parser::ast::types::Primitive::Unit)
    ) {
        return Err(eyre::eyre!(
            "main must return unit; use print or println for output"
        ));
    }
    if !has_main {
        warn!("no main function found");
    }

    let mut backend =
        vinyl_codegen::CraneliftBackend::new().map_err(|e| eyre::eyre!("jit init: {e}"))?;
    backend
        .compile(items)
        .map_err(|e| eyre::eyre!("jit compile: {e}"))?;
    backend.run().map_err(|e| eyre::eyre!("jit run: {e}"))?;
    Ok(())
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    miette::set_hook(Box::new(|_| Box::new(RustcReportHandler)))
        .map_err(|error| eyre::eyre!(error.to_string()))?;

    match cli.command {
        Command::Check { file } => {
            let file = file.unwrap_or_else(|| {
                if Path::new("src").exists() {
                    PathBuf::from("src")
                } else {
                    PathBuf::from(".")
                }
            });
            let _items = compile_and_report(&file)?;
        }
        Command::Run { file } => {
            let file = file.unwrap_or_else(|| {
                if Path::new("src").exists() {
                    PathBuf::from("src")
                } else {
                    PathBuf::from(".")
                }
            });
            let items = compile_and_report(&file)?;
            jit_and_run(&items)?;
        }
        Command::Build { file, output } => {
            let file = file.unwrap_or_else(|| {
                if Path::new("src").exists() {
                    PathBuf::from("src")
                } else {
                    PathBuf::from(".")
                }
            });
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
