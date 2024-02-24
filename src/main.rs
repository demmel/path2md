use std::{
    fs::File,
    io::{BufRead, BufReader},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use file_format::FileFormat;

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

    print_path(
        &args.path,
        &args.path,
        args.ignore.as_ref().map(|x| x.as_slice()),
    )?;

    Ok(())
}

fn print_path(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
    ignore: Option<&[glob::Pattern]>,
) -> Result<(), anyhow::Error> {
    let root_ref = root.as_ref();
    let path_ref = path.as_ref();

    if let Some(ignore) = ignore {
        for glob in ignore {
            if glob.matches_path(path_ref) {
                return Ok(());
            }
        }
    }

    let metadata = path_ref.metadata().with_context(|| {
        format!(
            "Failed to read metadata for \"{}\"",
            path.as_ref().to_string_lossy()
        )
    })?;

    if metadata.is_dir() {
        print_directory(root_ref, path_ref, ignore)?;
    } else if metadata.is_file() {
        print_file(root_ref, path_ref)?;
    } else if metadata.is_symlink() {
        anyhow::bail!("Symlinks should be unreachable when using std::fs::metadata()")
    } else {
        anyhow::bail!("Unsupported path type")
    }

    Ok(())
}

fn print_directory(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
    ignore: Option<&[glob::Pattern]>,
) -> Result<(), anyhow::Error> {
    let mut dir = path
        .as_ref()
        .read_dir()
        .with_context(|| format!("Failed to read dir \"{}\"", path.as_ref().to_string_lossy()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| {
            format!(
                "Failed to read dir entry in \"{}\"",
                path.as_ref().to_string_lossy(),
            )
        })?;

    dir.sort_by_key(|e| (!e.path().is_file(), e.path()));

    for entry in dir {
        let entry_path = entry.path();
        print_path(root.as_ref(), &entry_path, ignore)?;
    }

    Ok(())
}

fn print_file(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    let root_ref = root.as_ref();
    let path_ref = path.as_ref();

    println!("{}", strip_root(root_ref, path_ref)?);
    println!();

    let fmt = file_format::FileFormat::from_file(path_ref)?;

    if let FileFormat::PlainText = fmt {
        let file = BufReader::new(
            File::open(path_ref)
                .with_context(|| format!("Fialed to open \"{}\"", path_ref.to_string_lossy()))?,
        );

        for line in file.lines() {
            let line = line.with_context(|| {
                format!(
                    "Failed to read lien from \"{}\"",
                    path_ref.to_string_lossy(),
                )
            })?;
            println!("    {}", line.trim_end());
        }
    } else {
        let metadata = path_ref.metadata().with_context(|| {
            format!(
                "Fialed to read metadata for \"{}\"",
                path_ref.to_string_lossy()
            )
        })?;
        println!("    {} ({})", fmt.name(), fmt.media_type());
        println!("    ... {} bytes ...", metadata.file_size());
    }

    println!();
    println!();

    Ok(())
}

fn strip_root(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<String, anyhow::Error> {
    let root_str = root.as_ref().to_string_lossy();
    let path_str = path.as_ref().to_string_lossy();

    let stripped = path_str
        .strip_prefix(root_str.as_ref())
        .with_context(|| format!("\"{path_str}\" is not relative to \"{root_str}\""))?;

    Ok(stripped.to_string())
}
