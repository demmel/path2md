use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    os::windows::fs::MetadataExt,
    path::Path,
};

use anyhow::Context;
use file_format::FileFormat;

pub fn write_path_contents(
    writer: &mut impl Write,
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
        write_directory_contents(writer, root_ref, path_ref, ignore)?;
    } else if metadata.is_file() {
        write_file_contents(writer, root_ref, path_ref)?;
    } else if metadata.is_symlink() {
        anyhow::bail!("Symlinks should be unreachable when using std::fs::metadata()")
    } else {
        anyhow::bail!("Unsupported path type")
    }

    Ok(())
}

pub fn write_directory_contents(
    writer: &mut impl Write,
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
        write_path_contents(writer, root.as_ref(), &entry_path, ignore)?;
    }

    Ok(())
}

pub fn write_file_contents(
    writer: &mut impl Write,
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<(), anyhow::Error> {
    let root_ref = root.as_ref();
    let path_ref = path.as_ref();

    writeln!(writer, "{}", strip_root(root_ref, path_ref)?)?;
    writeln!(writer,)?;

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
            writeln!(writer, "    {}", line.trim_end())?;
        }
    } else {
        let metadata = path_ref.metadata().with_context(|| {
            format!(
                "Fialed to read metadata for \"{}\"",
                path_ref.to_string_lossy()
            )
        })?;
        writeln!(writer, "    {} ({})", fmt.name(), fmt.media_type())?;
        writeln!(writer, "    ... {} bytes ...", metadata.file_size())?;
    }

    writeln!(writer,)?;
    writeln!(writer,)?;

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
