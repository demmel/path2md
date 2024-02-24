use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;

/// Dump an the contents of a path to stdout in Markdown format
#[derive(clap::Parser)]
struct Args {
    /// The path to dump
    path: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    print_path(&args.path, &args.path)?;

    Ok(())
}

fn print_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    let root_ref = root.as_ref();
    let path_ref = path.as_ref();
    let metadata = path_ref.metadata().with_context(|| {
        format!(
            "Failed to read metadata for \"{}\"",
            path.as_ref().to_string_lossy()
        )
    })?;

    if metadata.is_dir() {
        print_directory(root_ref, path_ref)?;
    } else if metadata.is_file() {
        print_file(root_ref, path_ref)?;
    } else if metadata.is_symlink() {
        anyhow::bail!("Symlinks should be unreachable when using std::fs::metadata()")
    } else {
        anyhow::bail!("Unsupported path type")
    }

    Ok(())
}

fn print_directory(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    let dir = path
        .as_ref()
        .read_dir()
        .with_context(|| format!("Failed to read dir \"{}\"", path.as_ref().to_string_lossy()))?;

    for entry in dir {
        let entry = entry.with_context(|| {
            format!(
                "Failed to read dir entry in \"{}\"",
                path.as_ref().to_string_lossy(),
            )
        })?;

        let entry_path = entry.path();

        print_path(root.as_ref(), &entry_path)?;
    }

    Ok(())
}

fn print_file(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    println!("{}", strip_root(root.as_ref(), path.as_ref())?);

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
