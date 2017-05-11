# cargo-edit-locally

[![Build Status](https://travis-ci.org/alexcrichton/cargo-edit-locally.svg?branch=master)](https://travis-ci.org/alexcrichton/cargo-edit-locally)
[![Build status](https://ci.appveyor.com/api/projects/status/qx69c85cp1irk0ps?svg=true)](https://ci.appveyor.com/project/alexcrichton/cargo-edit-locally)

This is a [Cargo](http://doc.crates.io) subcommand which intends to allow easy
management of the `[replace]` section of Cargo.toml.

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

Let's say we've got a checkout of `log` locally and we'd like to verify it fixes
our bug:

```
$ cd $CODE
$ cargo edit-locally log --path ../log
$
```

And that's it! The local project, `foo`, is now configured to use the `log`
folder in our local code directory. We can see that `Cargo.toml` now has a
[`[replace]`][replace] section:

```toml
[replace]
'log:0.3.7' = { path = 'log' }
```

And finally can now use the `log` crate from our local build!

```
$ cargo build
   Compiling log v0.3.7 ($CODE/log)
   Compiling foo v0.1.0 ($CODE)
    Finished dev [unoptimized + debuginfo] target(s) in 1.97 secs
```

If we instead would like to test out a git repository we can use:

```
$ cargo edit-locally log --git https://github.com/rust-lang-nursery/log
```

To see a full suite of options available to you and another help message, execute:

```
$ cargo help edit-locally
```

## Undoing local edits

To go back to using crates.io, you can simply delete the `[replace]` section in
the manifest. This'll go back to using the version in the lock file, and the
next `cargo build` will compile code from crates.io instead of your local
folder.

After the `[replace]` section is deleted you can delete the folder of the
checkout as well, after saving off your work if needed.

## Caveats

This subcommand will automatically attempt to edit `Cargo.toml` and insert a
`[replace]` section for you. Unfortunately there's no great robust way right now
to edit TOML file preserving formatting and comments and such, so right now
there's mostly just a few heuristics to do this automatically.

If you find that the heuristics don't work for you though please let me know and
I'll try to check in a fix!

# License

`cargo-edit-locally` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), with portions covered by various
BSD-like licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
