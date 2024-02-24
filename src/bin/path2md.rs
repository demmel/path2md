use std::path::PathBuf;

use clap::Parser;

/// Dump an the contents of a path to stdout in Markdown format
#[derive(Debug, Parser)]
struct Args {
    /// The path to dump
    path: PathBuf,
    /// File globs to ignore
    #[clap(short, long, value_delimiter=',', value_parser=parse_globs)]
    ignore: Option<Vec<glob::Pattern>>,
}

fn parse_globs(s: &str) -> Result<glob::Pattern, glob::PatternError> {
    glob::Pattern::new(s)
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    path2md::Path2Md::new(args.path)
        .ignore(args.ignore)
        .write(&mut std::io::stdout())?;

    Ok(())
}
