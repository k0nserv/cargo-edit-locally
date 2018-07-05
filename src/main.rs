extern crate cargo;
extern crate docopt;
extern crate env_logger;
extern crate pathdiff;
extern crate toml;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;

use cargo::core::{Dependency, Source};
use cargo::core::{Workspace, SourceId, GitReference};
use cargo::ops;
use cargo::util::errors::*;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::paths;
use cargo::util::{Config, ToUrl};
use toml::Value;
use docopt::Docopt;

#[derive(Deserialize)]
struct Options {
    arg_spec: String,

    flag_path: Option<String>,
    flag_git: Option<String>,
    flag_branch: Option<String>,
    flag_tag: Option<String>,
    flag_rev: Option<String>,
    flag_manifest_path: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
}

fn main() {
    env_logger::init();
    let mut config = Config::default().unwrap();
    let usage = r#"
Configure the [replace] section in Cargo.toml to edit a dependency locally

Usage:
    cargo edit-locally [options] <spec>
    cargo edit-locally (--help | -h)

Options:
    --path PATH              Replace the package specified with a crate at PATH
    --git REPO               Replace the package specified with REPO
    --branch BRANCH          If replacing with a git repo, branch to check out
    --tag TAG                If replacing with a git reop, tag to check out
    --rev REV                If replacing with a git repo, revision to check out
    --manifest-path PATH     Path to the manifest to replace a dependency for
    -h, --help               Print this message
    -v, --verbose ...        Use verbose output
    -q, --quiet              No output printed to stdout
    --color WHEN             Coloring: auto, always, never

Rust projects tend to have a number of dependencies, and many of those may be
under active development! This subcommand is intended to ease the development
of such dependencies by making it easy to manage the [replace] section in
Cargo.toml and edit dependencies locally.

The `<spec>` argument is a package ID specification, and you can read more about
it with `cargo help pkgid`. Typically it's just the name of a crate, and it
specifies the crate that's being replaced. The flags passed to this command then
indicate what the crate is being replaced with, namely `--path` for a locally
checked out crate or `--git` for replacing with a git repository.

Some example invocations are:

    # Replace `log` with the crate's master branch
    cargo edit-locally log --git https://github.com/rust-lang-nursery/log

    # Replace `cargo` with a locally checked out copy at `../cargo`
    cargo edit-locally cargo --path ../cargo

If you have any questions about how to use this subcommand or would like to
see a new feature, please feel free to open an issue at
https://github.com/alexcrichton/cargo-edit-locally
"#;
    let options = Docopt::new(usage)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let result = real_main(options, &mut config);
    if let Err(e) = result {
        cargo::exit_with_error(e.into(), &mut *config.shell());
    }
}

fn real_main(options: Options, config: &mut Config) -> CargoResult<()> {
    config.configure(options.flag_verbose,
                     options.flag_quiet,
                     &options.flag_color,
                     /* frozen = */ false,
                     /* locked = */ false,
                     /* target_dir = */ &None,
                     /* unstable features = */ &[])?;

    // Load up and resolve the crate. This'll do the whole 'Updateing registry'
    // thing in Cargo, creating a lock file if one doesn't exist or reading it
    // if it does.
    let manifest = match options.flag_manifest_path {
        Some(path) => path.into(),
        None => find_root_manifest_for_wd(config.cwd())?,
    };
    let ws = Workspace::new(&manifest, config)?;
    let (_packages, resolve) = cargo::ops::resolve_ws(&ws).chain_err(|| {
        "failed resolve crate"
    })?;

    let to_replace = resolve.query(&options.arg_spec)?;

    let replace_with = if let Some(p) = options.flag_path {
        let path = paths::normalize_path(&config.cwd().join(p));
        SourceId::for_path(&path)?
    } else {
        let url = options.flag_git.ok_or_else(|| {
            format_err!("either --git or --path must be specified")
        })?.to_url()?;
        let reference = if let Some(b) = options.flag_branch {
            GitReference::Branch(b)
        } else if let Some(t) = options.flag_tag {
            GitReference::Tag(t)
        } else if let Some(r) = options.flag_rev {
            GitReference::Rev(r)
        } else {
            GitReference::Branch("master".to_string())
        };
        SourceId::for_git(&url, reference)?
    };

    let mut source = replace_with.load(config)?;
    source.update()?;

    let req = format!("={}", to_replace.version().to_string());
    let dependency = Dependency::parse_no_deprecated(&to_replace.name(),
                                                     Some(&req),
                                                     &replace_with)?;
    let candidates = source.query_vec(&dependency)?;
    if candidates.len() == 0 {
        let mut msg = format!("failed to find `{} v{}` inside of `{}`\n",
                              to_replace.name(),
                              to_replace.version(),
                              replace_with);
        if replace_with.is_git() {
            msg.push_str(&format!("perhaps a different branch/tag is needed?"));
        } else {
            msg.push_str(&format!("perhaps this path contains the wrong version?"));
        }
        bail!("{}", msg)
    }

    let crates_io = SourceId::crates_io(config)?;
    let to_replace_spec = if *to_replace.source_id() == crates_io {
        format!("{}:{}", to_replace.name(), to_replace.version())
    } else {
        format!("{}#{}:{}",
                to_replace.source_id().url(),
                to_replace.name(),
                to_replace.version())
    };

    let replace_line = if replace_with.is_git() {
        let git_extra = match *replace_with.git_reference().unwrap() {
            GitReference::Branch(ref s) if s == "master" => String::new(),
            GitReference::Branch(ref b) => format!(", branch = \"{}\"", b),
            GitReference::Tag(ref t) => format!(", tag = \"{}\"", t),
            GitReference::Rev(ref r) => format!(", rev = \"{}\"", r),
        };
        format!("{} = {{ git = {}{} }}\n",
                Value::String(to_replace_spec.clone()),
                Value::String(replace_with.url().to_string()),
                git_extra)
    } else {
        let absolute_path = replace_with.url().to_file_path().unwrap();
        let relative = pathdiff::diff_paths(&absolute_path, ws.root());
        let path;
        if let Some(ref buf) = relative {
            path = buf;
        } else {
            path = &absolute_path;
        }
        format!("{} = {{ path = {} }}\n",
                Value::String(to_replace_spec.clone()),
                Value::String(path.display().to_string()))
    };
    let manifest_path = ws.root().join("Cargo.toml");
    let mut manifest = paths::read(&manifest_path)?;

    match manifest.find("\n[replace]") {
        Some(i) => {
            match manifest[i + 1..].find("\n") {
                Some(j) => manifest.insert_str(i + 2 + j, &replace_line),
                None => {
                    manifest.push_str("\n");
                    manifest.push_str(&replace_line);
                }
            }
        }
        None => {
            if manifest.contains("[replace]") {
                bail!("don't know how to auto-modify `{}`",
                      manifest_path.display())
            }

            if manifest.split('\n').rev().take_while(|s| s.trim().is_empty()).count() == 0 {
                manifest.push_str("\n");
            }
            if manifest.split('\n').rev().take_while(|s| s.trim().is_empty()).count() == 1 {
                manifest.push_str("\n");
            }
            manifest.push_str("[replace]\n");
            manifest.push_str(&replace_line);
        }
    }

    paths::write(&manifest_path, manifest.as_bytes())?;

    // regenerate Cargo.lock
    let ws = Workspace::new(&manifest_path, config)?;
    ops::resolve_ws(&ws)?;

    Ok(())
}
