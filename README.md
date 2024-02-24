# path2md

Dump the contents of a path in Markdown format

## Use as a binary

```
Usage: path2md.exe [OPTIONS] <PATH>

Arguments:
  <PATH>  The path to dump

Options:
  -i, --ignore <IGNORE>  File globs to ignore
  -h, --help             Print help
```

## Use as a library

```
path2md::Path2Md::new("/some/path/".into())
    .write(&mut std::io::stdout())?;
```
