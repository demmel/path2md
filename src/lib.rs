pub use glob::Pattern;
use justerror::Error;

use std::{
    fs::{DirEntry, File},
    io::{BufRead, BufReader, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

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
}

#[Error]
pub enum Path2MdWriteError<E> {
    FailedToWalkPath(#[from] WalkPathError<E>),
}

impl Path2Md {
    pub fn write(
        &self,
        writer: &mut impl Write,
    ) -> Result<(), Path2MdWriteError<Path2MdWriteFileContentsError>> {
        walk_path_contents(
            &self.root,
            &|e| (!e.path().is_file(), e.path()),
            &|p| self.should_walk_path(p),
            &mut |p| {
                if p.is_file() {
                    self.write_file_contents(p, writer)?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}

#[Error]
pub enum Path2MdWriteFileContentsError {
    FailedToDisplayFilePath(#[from] StripRootError),
    FailedToWrite(std::io::Error),
    FailedToGetFileFormat(std::io::Error),
}

impl Path2Md {
    pub fn write_file_contents(
        &self,
        path: impl AsRef<Path>,
        writer: &mut impl Write,
    ) -> Result<(), Path2MdWriteFileContentsError> {
        let root_ref = &self.root;
        let path_ref = path.as_ref();

        let fmt = file_format::FileFormat::from_file(path_ref)
            .map_err(Path2MdWriteFileContentsError::FailedToGetFileFormat)?;
        let stripped_file_name = strip_root(root_ref, path_ref)?;

        let mut write_file_contents_inner = || -> Result<(), std::io::Error> {
            writeln!(writer, "{stripped_file_name}")?;
            writeln!(writer,)?;

            if let FileFormat::PlainText = fmt {
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

        write_file_contents_inner().map_err(Path2MdWriteFileContentsError::FailedToWrite)?;

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
    for_each: &mut impl FnMut(&Path) -> Result<(), E>,
) -> Result<(), WalkPathError<E>> {
    let path = path.as_ref();

    if !should_walk(path) {
        return Ok(());
    }

    for_each(path)?;

    if path.is_dir() {
        let mut dir = path
            .read_dir()
            .map_err(|e| WalkPathError::FailedToReadDir(e, path.to_path_buf()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WalkPathError::FailedToReadDirEntry(e, path.to_path_buf()))?;

        dir.sort_by_key(|e| order_by_key(e));

        for entry in dir {
            walk_path_contents(&entry.path(), order_by_key, should_walk, for_each)?;
        }
    }

    Ok(())
}

#[Error]
pub enum StripRootError {
    PrefixDoesntMatch(PathBuf, PathBuf),
}

fn strip_root(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<String, StripRootError> {
    let root_str = root.as_ref().to_string_lossy();
    let path_str = path.as_ref().to_string_lossy();

    let stripped =
        path_str
            .strip_prefix(root_str.as_ref())
            .ok_or(StripRootError::PrefixDoesntMatch(
                root.as_ref().to_path_buf(),
                path.as_ref().to_path_buf(),
            ))?;

    Ok(stripped.to_string())
}
