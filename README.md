# cargo-edit-locally

[![Build Status](https://travis-ci.org/alexcrichton/cargo-edit-locally.svg?branch=master)](https://travis-ci.org/alexcrichton/cargo-edit-locally)
[![Build status](https://ci.appveyor.com/api/projects/status/qx69c85cp1irk0ps?svg=true)](https://ci.appveyor.com/project/alexcrichton/cargo-edit-locally)

This is a [Cargo](http://doc.crates.io) subcommand which allows easily checking
out dependencies of a crate for local modification.

## Installation

Currently this can be installed with:

```
$ cargo install --git https://github.com/alexcrichton/cargo-edit-locally
```

You can also install [precompiled
binaries](https://github.com/alexcrichton/cargo-edit-locally/releases) that are
assembled on the CI for this crate.

## Example Usage

The `cargo edit-locally` command can be executed in any directory but it's
recommended to execute it in an existing project with a `Cargo.toml`

```
$ cargo edit-locally log
    Fetching metadata for `log`
     Cloning https://github.com/rust-lang/log
Dependency `log` has its source code now located at `log`.
To use this source code ensure that the following section is added
to `./Cargo.toml`

    [replace]
    'log:0.3.7' = { path = 'log' }


When you're done working with the source code then you can delete the `[replace]` section entry
```

In this example our project has a dependency on the [`log`] crate which we'd
like to make edits to. After this command executes there is a `log` directory
in the current directory which is a git checkout of the `log` crate pinned to
version 0.3.7.

The subcommand then prints an appropriate [`[replace]` section][replace] to
paste into the specified `Cargo.toml`. After inserted Cargo will use the source
code locally for edits.

[replace]: http://doc.crates.io/manifest.html#the-replace-section

A second optional argument is accepted to check out the crate to a different
location:

```
$ cargo edit-locally log ../crates
```

[`log`]: https://crates.io/crates/log

## Git repositories and crates.io

Crates published to crates.io are not guaranteed to have a git repository
behind them. This crate will check the `repository` field in the crate
specified. If found the git repository will be checked out and it will probe
for the correct commit. If not found it will simply copy the sources from
crates.io.

When looking for the right commit in a git repository this command will search
for tags or branches corresponding to the version denied, and failing that it
will traverse backwards through the repository's history looking for the first
commit with the version mentioned.

# License

`cargo-edit-locally` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), with portions covered by various
BSD-like licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
