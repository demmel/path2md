pub use glob::Pattern;

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::Context;
use file_format::FileFormat;

pub struct Path2Md {
    pub root: PathBuf,
    pub ignore: Option<Vec<Pattern>>,
}

impl Path2Md {
    pub fn new(root: PathBuf) -> Self {
        Self { root, ignore: None }
    }

    pub fn ignore(mut self, ignore: Option<Vec<glob::Pattern>>) -> Self {
        self.ignore = ignore;
        self
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<(), anyhow::Error> {
        self.write_path_contents(&self.root, writer)
    }

    pub fn write_path_contents(
        &self,
        path: impl AsRef<Path>,
        writer: &mut impl Write,
    ) -> Result<(), anyhow::Error> {
        let path_ref = path.as_ref();

        if let Some(ignore) = &self.ignore {
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
            self.write_directory_contents(path_ref, writer)?;
        } else if metadata.is_file() {
            self.write_file_contents(path_ref, writer)?;
        } else if metadata.is_symlink() {
            anyhow::bail!("Symlinks should be unreachable when using std::fs::metadata()")
        } else {
            anyhow::bail!("Unsupported path type")
        }

        Ok(())
    }

    pub fn write_directory_contents(
        &self,
        path: impl AsRef<Path>,
        writer: &mut impl Write,
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
            self.write_path_contents(&entry_path, writer)?;
        }

        Ok(())
    }

    pub fn write_file_contents(
        &self,
        path: impl AsRef<Path>,
        writer: &mut impl Write,
    ) -> Result<(), anyhow::Error> {
        let root_ref = &self.root;
        let path_ref = path.as_ref();

        writeln!(writer, "{}", strip_root(root_ref, path_ref)?)?;
        writeln!(writer,)?;

        let fmt = file_format::FileFormat::from_file(path_ref)?;

        if let FileFormat::PlainText = fmt {
            let file =
                BufReader::new(File::open(path_ref).with_context(|| {
                    format!("Fialed to open \"{}\"", path_ref.to_string_lossy())
                })?);

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
}

fn strip_root(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<String, anyhow::Error> {
    let root_str = root.as_ref().to_string_lossy();
    let path_str = path.as_ref().to_string_lossy();

    let stripped = path_str
        .strip_prefix(root_str.as_ref())
        .with_context(|| format!("\"{path_str}\" is not relative to \"{root_str}\""))?;

    Ok(stripped.to_string())
}
