pub use glob::Pattern;

use std::{
    fs::{DirEntry, File},
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
        walk_path_contents(
            &self.root,
            |e| (!e.path().is_file(), e.path()),
            |p| self.should_walk_path(p),
            |p| {
                if p.is_file() {
                    self.write_file_contents(p, writer)?;
                }
                Ok(())
            },
        )
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

    fn should_walk_path(&self, path: impl AsRef<Path>) -> bool {
        if let Some(ignore) = &self.ignore {
            for glob in ignore {
                if glob.matches_path(path.as_ref()) {
                    return false;
                }
            }
        }
        true
    }
}

fn walk_path_contents<K: Ord>(
    path: impl AsRef<Path>,
    order_by_key: impl Fn(&DirEntry) -> K,
    should_walk: impl Fn(&Path) -> bool,
    mut for_each: impl FnMut(&Path) -> Result<(), anyhow::Error>,
) -> Result<(), anyhow::Error> {
    let path = path.as_ref();

    if !should_walk(path) {
        return Ok(());
    }

    for_each(path)?;

    if path.is_dir() {
        let mut dir = path
            .read_dir()
            .with_context(|| format!("Failed to read dir \"{}\"", path.to_string_lossy()))?
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| {
                format!("Failed to read dir entry in \"{}\"", path.to_string_lossy(),)
            })?;

        dir.sort_by_key(|e| order_by_key(e));

        for entry in dir {
            walk_path_contents(&entry.path(), &order_by_key, &should_walk, &mut for_each)?;
        }
    }

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
