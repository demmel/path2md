pub use glob::Pattern;
use justerror::Error;

use std::{
    fs::{DirEntry, File},
    io::{BufRead, BufReader, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub struct Path2Md {
    pub root: PathBuf,
    pub ignore: Option<Vec<Pattern>>,
    pub structure_only: bool,
}

impl Path2Md {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            structure_only: false,
            ignore: None,
        }
    }

    pub fn ignore(mut self, ignore: Option<Vec<glob::Pattern>>) -> Self {
        self.ignore = ignore;
        self
    }

    pub fn structure_only(mut self, structure_only: bool) -> Self {
        self.structure_only = structure_only;
        self
    }

    fn should_walk_path(&self, path: impl AsRef<Path>) -> bool {
        if let Some(ignore) = &self.ignore {
            for glob in ignore {
                if glob.matches_path(path.as_ref().strip_prefix(&self.root).unwrap()) {
                    return false;
                }
            }
        }
        true
    }
}

#[Error]
pub enum Path2MdWriteError {
    FailedToWriteFileContents(#[from] WalkPathError<Path2MdWriteFileContentsError>),
    FailedToWriteStructure(#[from] WalkPathError<Path2MdWriteStructureError>),
    FailedToWrite(#[from] std::io::Error),
}

impl Path2Md {
    pub fn write(&self, writer: &mut impl Write) -> Result<(), Path2MdWriteError> {
        if self.root.is_dir() {
            writeln!(writer, "# Directory Structure")?;
            writeln!(writer)?;
            walk_path_contents(
                &self.root,
                &|e| (!e.path().is_dir(), e.path()),
                &|p| self.should_walk_path(p),
                &mut |p, is_last| self.write_structure_line(p, is_last, writer),
            )?;
            writeln!(writer)?;
            writeln!(writer)?;
        }

        if !self.structure_only {
            walk_path_contents(
                &self.root,
                &|e| (!e.path().is_file(), e.path()),
                &|p| self.should_walk_path(p),
                &mut |p, _| {
                    if p.is_file() {
                        self.write_file_contents(p, writer)?;
                    }
                    Ok::<_, Path2MdWriteFileContentsError>(())
                },
            )?;
        }
        Ok(())
    }
}

#[Error]
pub enum Path2MdWriteStructureError {
    FailedToWrite(#[from] std::io::Error),
}

impl Path2Md {
    fn write_structure_line(
        &self,
        path: &Path,
        is_last: bool,
        writer: &mut impl Write,
    ) -> Result<(), Path2MdWriteStructureError> {
        let path = path.strip_prefix(&self.root).unwrap();

        let suffix = if let Some(c) = path.components().last() {
            c.as_os_str().to_string_lossy().to_string()
        } else {
            ".".to_string()
        };

        let depth = path.components().count();
        write!(writer, "    ")?;
        for _ in 0..(depth.saturating_sub(1)) {
            write!(writer, "│ ")?;
        }
        if depth != 0 {
            if !is_last {
                write!(writer, "├─")?;
            } else {
                write!(writer, "└─")?;
            }
        }
        writeln!(writer, "{}", suffix)?;
        Ok(())
    }
}

#[Error]
pub enum Path2MdWriteFileContentsError {
    FailedToDisplayFilePath(#[from] StripRootError),
    FailedToWrite(PathBuf, std::io::Error),
    FailedToGetFileFormat(PathBuf, std::io::Error),
}

impl Path2Md {
    pub fn write_file_contents(
        &self,
        path: impl AsRef<Path>,
        writer: &mut impl Write,
    ) -> Result<(), Path2MdWriteFileContentsError> {
        let path_ref = path.as_ref();

        let fmt = file_format::FileFormat::from_file(path_ref).map_err(|e| {
            Path2MdWriteFileContentsError::FailedToGetFileFormat(path_ref.to_path_buf(), e)
        })?;
        let mut stripped_path = path_ref.strip_prefix(&self.root).unwrap();
        if stripped_path.components().count() == 0 {
            stripped_path = &self.root;
        }

        let mut write_file_contents_inner = || -> Result<(), std::io::Error> {
            writeln!(writer, "# {}", stripped_path.to_string_lossy())?;
            writeln!(writer,)?;

            if fmt.media_type().starts_with("text") {
                let file = BufReader::new(File::open(path_ref)?);
                for line in file.lines() {
                    let line = line?;
                    writeln!(writer, "    {}", line.trim_end())?;
                }
            } else {
                let metadata = path_ref.metadata()?;
                writeln!(writer, "    {} ({})", fmt.name(), fmt.media_type())?;
                writeln!(writer, "    ... {} bytes ...", metadata.file_size())?;
            }

            writeln!(writer,)?;
            writeln!(writer,)?;

            Ok(())
        };

        write_file_contents_inner()
            .map_err(|e| Path2MdWriteFileContentsError::FailedToWrite(path_ref.to_path_buf(), e))?;

        Ok(())
    }
}

#[Error]
pub enum WalkPathError<E> {
    FailedToReadDir(std::io::Error, PathBuf),
    FailedToReadDirEntry(std::io::Error, PathBuf),
    ForEachFailed(#[from] E),
}

fn walk_path_contents<K: Ord, E>(
    path: impl AsRef<Path>,
    order_by_key: &impl Fn(&DirEntry) -> K,
    should_walk: &impl Fn(&Path) -> bool,
    for_each: &mut impl FnMut(&Path, bool) -> Result<(), E>,
) -> Result<(), WalkPathError<E>> {
    walk_path_contents_helper(path, order_by_key, should_walk, for_each, true)
}

fn walk_path_contents_helper<K: Ord, E>(
    path: impl AsRef<Path>,
    order_by_key: &impl Fn(&DirEntry) -> K,
    should_walk: &impl Fn(&Path) -> bool,
    for_each: &mut impl FnMut(&Path, bool) -> Result<(), E>,
    is_last: bool,
) -> Result<(), WalkPathError<E>> {
    let path = path.as_ref();

    if !should_walk(path) {
        return Ok(());
    }

    for_each(path, is_last)?;

    if path.is_dir() {
        let mut dir = path
            .read_dir()
            .map_err(|e| WalkPathError::FailedToReadDir(e, path.to_path_buf()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WalkPathError::FailedToReadDirEntry(e, path.to_path_buf()))?;

        dir.sort_by_key(|e| order_by_key(e));

        let len = dir.len();
        for (i, entry) in dir.into_iter().enumerate() {
            walk_path_contents_helper(
                &entry.path(),
                order_by_key,
                should_walk,
                for_each,
                i == len - 1,
            )?;
        }
    }

    Ok(())
}

#[Error]
pub enum StripRootError {
    PrefixDoesntMatch(PathBuf, PathBuf),
}
