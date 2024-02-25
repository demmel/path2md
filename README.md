# path2md

Dump the contents of a path in Markdown format

Have you ever wanted to recursively dump the contents of a directory serially as markdown formatters codeblocks? Now you can!

Let's say you have the following dircetory structure and contents.

```
# Directory Structure

    .
    ├─src
    │ └─main.rs
    ├─.gitignore
    └─Cargo.toml


# .gitignore

    /target


# Cargo.toml

    [package]
    name = "empty"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]


# src\main.rs

    fn main() {
        println!("Hello, world!");
    }
```

path2md outputs exactly that.

## Use as a binary

```
Dump an the contents of a path to stdout in Markdown format

Usage: path2md.exe [OPTIONS] <PATH>

Arguments:
  <PATH>  The path to dump

Options:
  -i, --ignore <IGNORE>  File globs to ignore
  -s, --structure-only   Only output the directory structure
  -h, --help             Print help
```

## Use as a library

Disable the `cli` feature if using as a library. It's only needed for the binary. I'd disable it by default for you, but this makes the ergonomics of `cargo install` painful because cargo doesn't support automatically enabling `required-features` for a bin target. :P

```
path2md::Path2Md::new("/some/path/".into())
    .write(&mut std::io::stdout())?;
```
