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

Disable the `cli` feature if using as a library. It's only needed for the binary. I'd disable it by default for you, but this makes the ergonomics of `cargo install` painful because cargo doesn't support automatically enabling `required-features` for a bin target. :P

```
path2md::Path2Md::new("/some/path/".into())
    .write(&mut std::io::stdout())?;
```
