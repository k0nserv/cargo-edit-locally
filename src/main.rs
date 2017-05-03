extern crate cargo;
extern crate curl;
extern crate env_logger;
extern crate git2;
extern crate rustc_serialize;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::str;

use cargo::CliResult;
use cargo::core::{Workspace, PackageId, PackageSet};
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::{human, ChainError, Config, CargoResult};
use cargo::util::paths;
use cargo::util::network;
use cargo::sources::git::{self, GitRemote};

macro_rules! bail {
    ($($fmt:tt)*) => (
        return Err(::human(&format_args!($($fmt)*)).into())
    )
}

#[derive(RustcDecodable)]
struct Options {
    arg_spec: String,
    arg_path: Option<String>,

    flag_manifest_path: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
}

fn main() {
    env_logger::init().unwrap();
    let config = Config::default().unwrap();
    let args = env::args().collect::<Vec<_>>();
    let result = cargo::call_main_without_stdin(real_main, &config, r#"
Check out a dependency in a local directory for modifications

Usage:
    cargo edit-locally [options] <spec> [<path>]

Options:
    --manifest-path PATH     Path to the manifest which lists dependencies
    -h, --help               Print this message
    -v, --verbose ...        Use verbose output
    -q, --quiet              No output printed to stdout
    --color WHEN             Coloring: auto, always, never

Rust projects tend to have a number of dependencies, and many of those may be
under active development! This subcommand is intended to ease the development
of such dependencies by making it easy to check out a dependency locally, make
modifications, and test them.

This crate is fundamentally built on the `[replace]` feature in Cargo, allowing
dependencies to get overridden with an equivalent copy. This command will
check out the crate via an appropriate mechanism to a local path and then
print out a `[replace]` section to add to `Cargo.toml`. Once added then local
builds will use the checked out source.

The `<spec>` argument is a package ID specification, and you can read more about
it with `cargo help pkgid`. Typically it's just the name of a crate.

The `<path>` argument is where crates will be checked out into. The `<path>`
must be a directory which will be filled in with a folder corresponding to the
crate's name, filled in with the source code under that.

Some example invocations are:

    # Check out the `log` crate's source code from crates.io into a local folder
    # named `log`
    cargo edit-locally log

    # Check out the `cargo` crate into a `foo/cargo`
    cargo edit-locally cargo ./foo

If you have any questions about how to use this subcommand or would like to
see a new feature, please feel free to open an issue at
https://github.com/alexcrichton/cargo-edit-locally
"#, &args, false);

    if let Err(e) = result {
        cargo::exit_with_error(e, &mut *config.shell());
    }
}

fn real_main(options: Options, config: &Config) -> CliResult {
    config.configure(options.flag_verbose,
                     options.flag_quiet,
                     &options.flag_color,
                     /* frozen = */ false,
                     /* locked = */ false)?;

    // Figure out where we'll be writing files and validate it.
    let destination = match options.arg_path {
        Some(ref s) => s.as_ref(),
        None => config.cwd(),
    };
    if !destination.exists() {
        fs::create_dir_all(&destination).chain_error(|| {
            human("failed to create destination directory")
        })?;
    }
    if !destination.is_dir() {
        bail!("destination must be a directory");
    }

    // Load up and resolve the crate. This'll do the whole 'Updateing registry'
    // thing in Cargo, creating a lock file if one doesn't exist or reading it
    // if it does.
    let manifest = find_root_manifest_for_wd(options.flag_manifest_path,
                                             config.cwd())?;
    let ws = Workspace::new(&manifest, config)?;
    let (packages, resolve) = cargo::ops::resolve_ws(&ws).chain_error(|| {
        human("failed resolve crate")
    })?;

    let id = resolve.query(&options.arg_spec)?;
    let destination = destination.join(id.name());

    // Basic sanity checks about the destination directory.
    if id.source_id().is_path() {
        bail!("{} is already a path dependency to edit locally", id);
    }
    if resolve.replacements().contains_key(id) {
        bail!("{} is already replaced, cannot replace it again", id);
    }
    if destination.exists() {
        bail!("looks like the destination directory for this checkout already \
               exists: {}", destination.display());
    }

    if id.source_id().is_git() {
        git_clone(id, &destination, config).chain_error(|| {
            human("failed to clone dependency")
        })?;
    } else {
        registry_clone(id, &destination, &packages, config).chain_error(|| {
            human("failed to clone dependency")
        })?;
    }

    if !options.flag_quiet.unwrap_or(false) {
        print_pretty_success(id, &destination, &ws, config)?;
    }

    Ok(())
}

fn print_pretty_success(id: &PackageId,
                        destination: &Path,
                        ws: &Workspace,
                        config: &Config) -> CargoResult<()> {
    let pretty_path = destination.strip_prefix(config.cwd())
                                 .unwrap_or(&destination);
    let mut manifest_dir = ws.root().strip_prefix(config.cwd())
                                    .unwrap_or(ws.root());
    if manifest_dir.parent().is_none() {
        manifest_dir = Path::new(".");
    }
    let manifest_to_path = destination.strip_prefix(ws.root())
                                      .unwrap_or(&destination);

    let pkgid = if id.source_id().is_default_registry() {
        format!("{}:{}", id.name(), id.version())
    } else {
        format!("{}#{}:{}", id.source_id().url(), id.name(), id.version())
    };

    let manifest = paths::read(&manifest_dir.join("Cargo.toml"))?;

    println!("Dependency `{}` has its source code now located at `{}`.",
             id.name(),
             pretty_path.display());
    if manifest.contains("[replace]") {
        println!("\
To use this source code ensure that the following section is added
to `{manifest_dir}/Cargo.toml` inside of the existing `[replace]` section

    '{pkgid}' = {{ path = '{manifest_to_path}' }}

",
            manifest_dir = manifest_dir.display(),
            pkgid = pkgid,
            manifest_to_path = manifest_to_path.display()
        );
    } else {
        println!("\
To use this source code ensure that the following section is added
to `{manifest_dir}/Cargo.toml`

    [replace]
    '{pkgid}' = {{ path = '{manifest_to_path}' }}

",
            manifest_dir = manifest_dir.display(),
            pkgid = pkgid,
            manifest_to_path = manifest_to_path.display(),
        );
    }

    println!("When you're done working with the source code then you can \
              delete the `[replace]` section entry");

    Ok(())
}

fn git_clone(id: &PackageId,
             destination: &Path,
             config: &Config) -> CargoResult<()> {
    let repo = git2::Repository::init(destination).chain_error(|| {
        human(format!("failed to initialize git repo at {}",
                      destination.display()))
    })?;

    config.shell().status("Cloning", id.source_id().url())?;
    let reference = "refs/heads/*:refs/heads/*";
    git::fetch(&repo,
               &id.source_id().url().to_string(),
               reference,
               config).chain_error(|| {
        human(format!("failed to fetch reference `{}`", reference))
    })?;

    let remote = GitRemote::new(id.source_id().url());
    let rev = remote.rev_for(&destination,
                             id.source_id().git_reference().unwrap())?.to_string();
    let id = rev.parse().unwrap();
    let object = repo.find_object(id, None)?;
    repo.reset(&object, git2::ResetType::Hard, None)?;

    Ok(())
}

fn registry_clone(id: &PackageId,
                  destination: &Path,
                  packages: &PackageSet,
                  config: &Config) -> CargoResult<()> {
    let mut handle = curl::easy::Easy::new();
    let url = format!("https://crates.io/api/v1/crates/{}", id.name());
    handle.get(true)?;
    handle.url(&url)?;
    handle.useragent("cargo-edit-locally")?;
    let mut headers = curl::easy::List::new();
    headers.append("Accept: application/json")?;
    handle.http_headers(headers)?;


    config.shell().status("Fetching", format!("metadata for `{}`", id.name()))?;
    let mut response = Vec::new();
    network::with_retry(config, || {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            response.extend_from_slice(data);
            Ok(data.len())
        })?;
        transfer.perform()
    }).chain_error(|| {
        human("failed to download crate metadata")
    })?;

    if handle.response_code()? != 200 {
        bail!("failed to get 200, got {}", handle.response_code()?);
    }

    let json: serde_json::Value = serde_json::from_slice(&response)?;
    match json["crate"].get("repository") {
        Some(repo) => {
            let repo = repo.as_str().unwrap();
            registry_clone_git(id, destination, repo, config)
        }
        None => registry_clone_fallback(id, destination, packages, config),
    }
}

fn registry_clone_git(id: &PackageId,
                      destination: &Path,
                      repo_url: &str,
                      config: &Config) -> CargoResult<()> {
    let repo = git2::Repository::init(destination).chain_error(|| {
        human(format!("failed to initialize git repo at {}",
                      destination.display()))
    })?;

    config.shell().status("Cloning", repo_url)?;
    let reference = "refs/heads/*:refs/heads/*";
    git::fetch(&repo, repo_url, reference, config).chain_error(|| {
        human(format!("failed to fetch reference `{}`", reference))
    })?;

    let mut candidates = Vec::new();
    candidates.push(id.version().to_string());
    candidates.push(format!("v{}", id.version()));
    candidates.push(format!("{}-{}", id.name(), id.version()));
    candidates.push(format!("{}-v{}", id.name(), id.version()));

    for c in candidates.iter() {
        let tagref = format!("refs/tags/{}", c);
        let tag = repo.refname_to_id(&tagref).and_then(|id| {
            repo.find_object(id, None)
        }).and_then(|object| {
            object.peel(git2::ObjectType::Commit)
        }).ok();

        let branch = repo.find_branch(&c, git2::BranchType::Local).ok();
        let branch = branch.and_then(|b| {
            b.get().target()
        }).and_then(|id| {
            repo.find_object(id, None).ok()
        });

        let object = match tag.or(branch) {
            Some(object) => object,
            None => continue,
        };
        repo.reset(&object, git2::ResetType::Hard, None)?;
        return Ok(())
    }

    config.shell().warn(format!("failed to find tag or branch for `{}`, probing \
                                 git history", id.version()))?;

    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::SORT_TOPOLOGICAL);
    walk.set_sorting(git2::SORT_TIME);
    let head = repo.refname_to_id("refs/heads/master")?;
    walk.push(head)?;

    let version = id.version().to_string();
    let valid_manifest = |manifest: &Manifest| {
        manifest.name == id.name() && manifest.version == version
    };
    for id in walk {
        let id = id?;
        let commit = repo.find_commit(id)?;
        let tree = commit.tree()?;

        if let Some(entry) = tree.get_name("Cargo.toml") {
            if let Ok(manifest) = to_manifest(&repo, &entry) {
                if valid_manifest(&manifest) {
                    repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;
                    return Ok(())
                }
            }
        }

        if find_manifest(&repo, &tree, &valid_manifest)? {
            repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;
            return Ok(())
        }
    }

    bail!("\
failed to check out the crate `{}` at a revision for the version `{}`
after cloning `{}` into: {}

  * no branch or tag found with the names: {}
  * no commit found with a `Cargo.toml` that contains `version = '{}'`

please file an issue with cargo-edit-locally if you believe this message is in
error
",
    id.name(),
    id.version(),
    repo_url,
    destination.display(),
    candidates.join(", "),
    id.version())
}

fn find_manifest(repo: &git2::Repository,
                 tree: &git2::Tree,
                 valid_manifest: &Fn(&Manifest) -> bool)
                 -> CargoResult<bool> {

    for entry in tree.iter() {
        match entry.kind() {
            Some(git2::ObjectType::Blob) => {}
            Some(git2::ObjectType::Tree) => {
                let object = entry.to_object(repo)?;
                let tree = object.as_tree().unwrap();
                if find_manifest(repo, tree, valid_manifest)? {
                    return Ok(true)
                }
                continue
            }
            _ => continue,
        }

        let name = match entry.name() {
            Some(s) => s,
            None => continue,
        };
        let name = Path::new(name);
        if !name.ends_with("Cargo.toml") {
            continue
        }
        if let Ok(manifest) = to_manifest(&repo, &entry) {
            if valid_manifest(&manifest) {
                return Ok(true)
            }
        }
    }

    Ok(false)
}

#[derive(Deserialize, Debug)]
struct Package {
    package: Option<Manifest>,
    project: Option<Manifest>,
}

#[derive(Deserialize, Debug)]
struct Manifest {
    name: String,
    version: String,
}

fn to_manifest(repo: &git2::Repository, entry: &git2::TreeEntry)
            -> CargoResult<Manifest> {
    let object = entry.to_object(&repo)?;
    let blob = object.as_blob().chain_error(|| {
        human("tree entry was not a blob")
    })?;
    let s = str::from_utf8(blob.content()).map_err(|_| {
        human("non-utf8 bytes")
    })?;
    toml::from_str::<Package>(s).map_err(|e| {
        human(format!("failed to parse toml: {}", e))
    }).map(|p| {
        p.package.or(p.project).unwrap()
    })
}

fn registry_clone_fallback(id: &PackageId,
                           destination: &Path,
                           packages: &PackageSet,
                           config: &Config) -> CargoResult<()> {
    let msg = format!("no repository listed for `{}` in crates.io metadata, \
                       falling back to copying files directly from crates.io; \
                       note that this may not work for all crates, and please \
                       file an issue with cargo-edit-locally if it ends up \
                       not working",
                      id.name());
    config.shell().warn(msg)?;

    let pkg = packages.get(id)?;
    cp_r(pkg.root(), destination)?;
    Ok(())
}

fn cp_r(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir(dst)?;
    for entry in src.read_dir()? {
        let entry = entry?;

        let src = entry.path();
        let dst = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            cp_r(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}
