use std::path::PathBuf;

use clap::Parser;
use path2md::write_path_contents;

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

    write_path_contents(
        &mut std::io::stdout(),
        &args.path,
        &args.path,
        args.ignore.as_ref().map(|x| x.as_slice()),
    )?;

    Ok(())
}
