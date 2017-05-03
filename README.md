# cargo-edit-locally

[![Build Status](https://travis-ci.org/alexcrichton/cargo-edit-locally.svg?branch=master)](https://travis-ci.org/alexcrichton/cargo-edit-locally)
[![Build status](https://ci.appveyor.com/api/projects/status/qx69c85cp1irk0ps?svg=true)](https://ci.appveyor.com/project/alexcrichton/cargo-edit-locally)

This is a [Cargo](http://doc.crates.io) subcommand which allows easily checking
out dependencies of a crate for local modification.

## Installation

Currently this can be installed with:

```
$ cargo install cargo-edit-locally
```

You can also install [precompiled
binaries](https://github.com/alexcrichton/cargo-edit-locally/releases) that are
assembled on the CI for this crate.

## Example Usage

After working on some Rust code for a bit let's say that we've got a dependency
on the `log` crate on crates.io. We think we've found a bug in the `log` crate
so we'd like to test out our findings and check it out. First, let's take a look
at our `$CODE/Cargo.toml`:

```toml
[package]
name = "foo"
version = "0.1.0"

[dependencies]
log = "0.3"
```

First up we need to invoke `cargo edit-locally`:

```
$ cd $CODE
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

You'll find that there's now a git repository at `$CODE/log`. This code is a
checked out version of the source code for the `log` crate on crates.io, checked
out to the commit that was uploaded to crates.io.

After modifying our Cargo.toml to add our [`[replace]`][replace] section:

```toml
[replace]
'log:0.3.7' = { path = 'log' }
```

We can now use the `log` crate from our local build!

```
$ cargo build
   Compiling log v0.3.7 ($CODE/log)
   Compiling foo v0.1.0 ($CODE)
    Finished dev [unoptimized + debuginfo] target(s) in 1.97 secs
```

All local changes to the local `log` folder will show up immediately and are
tracked by `cargo build`. The git repository should also allow you to track
changes over time and even send a PR when ready!

To see a full suite of options available to you and another help message, execute:

```
$ cargo help edit-locally
```

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

Note that these heuristics are sort of best effort, it's recommended that you
run tests just after adding a `[replace]` section before modifications are
made. If it looks like the same source as before then you should be good to go.

## Undoing local edits

To go back to using crates.io, you can simply delete the `[replace]` section in
the manifest. This'll go back to using the version in the lock file, and the
next `cargo build` will compile code from crates.io instead of your local
folder.

After the `[replace]` section is deleted you can delete the folder of the
checkout as well, after saving off your work if needed.

# License

`cargo-edit-locally` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), with portions covered by various
BSD-like licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
