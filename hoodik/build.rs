use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// If no dist dir is available, we will just write an empty client
fn handle_no_dist(client_out_file: &mut File) -> io::Result<()> {
    writeln!(client_out_file, "pub(crate) const _DEFAULT: &[u8] = &[];",)?;

    writeln!(
        client_out_file,
        "pub(crate) const _CLIENT: [(&str, &[u8]); 0] = [];",
    )?;

    Ok(())
}

fn main() -> io::Result<()> {
    let client_dist_dir = PathBuf::from("../web/dist");

    // The generated table is a build artifact, so it goes to OUT_DIR and is
    // pulled in with `include!` from lib.rs. Writing into `src/` made the
    // source tree depend on the last build and breaks read-only checkouts.
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let mut client_out_file = File::create(out_dir.join("client.rs"))?;

    // Always emit the rerun-if-changed directive — without it, a first build
    // that ran before `web/dist/` existed would never re-run build.rs once
    // the Vite output finally lands, and the release binary would ship a
    // permanently-empty client embed.
    println!("cargo:rerun-if-changed=../web/dist");

    emit_client_compat();

    if !client_dist_dir.exists() {
        return handle_no_dist(&mut client_out_file);
    }

    let dist = client_dist_dir.canonicalize()?;

    let mut files = Vec::new();
    collect_files(&dist, &dist, &mut files)?;

    // `read_dir` order is filesystem dependent. Sorting keeps the generated
    // table, and therefore the binary, identical from one build to the next.
    files.sort();

    // Paths are written with `{:?}` so that quotes, backslashes and other
    // special characters come out as a valid Rust string literal on every
    // platform, Windows included.
    writeln!(
        client_out_file,
        "pub(crate) const _DEFAULT: &[u8] = include_bytes!({:?});",
        path_str(&dist.join("index.html"))
    )?;

    writeln!(
        client_out_file,
        "pub(crate) const _CLIENT: [(&str, &[u8]); {}] = [",
        files.len()
    )?;

    for (relative_path, absolute_path) in &files {
        writeln!(
            client_out_file,
            "({:?}, include_bytes!({:?})),",
            relative_path,
            path_str(absolute_path)
        )?;
    }

    writeln!(client_out_file, "];")?;

    Ok(())
}

/// Walk `dir` and collect every file as (path relative to `root` with `/`
/// separators, absolute path).
fn collect_files(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            let relative_path = path
                .strip_prefix(root)
                .expect("walked path is under the dist root")
                .components()
                .map(|c| path_str(Path::new(c.as_os_str())).to_owned())
                .collect::<Vec<_>>()
                .join("/");

            files.push((relative_path, path));
        }
    }
    Ok(())
}

fn path_str(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_else(|| panic!("non UTF-8 path in web/dist: {}", path.display()))
}

/// Lift `[package.metadata.compat]` out of Cargo.toml and into the binary.
///
/// Cargo does not hand `package.metadata` to the compiler, so it has to come
/// through here. Keeping the values in the manifest means a release touches
/// one file: the version and what it is compatible with sit together, rather
/// than the second hiding in a source constant nobody remembers to bump.
///
/// A missing or malformed table is a hard error. Silently emitting an empty
/// string would ship a server that tells every app it is compatible, which is
/// the one answer that cannot be recovered from in the field.
fn emit_client_compat() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    let manifest: toml::Value = std::fs::read_to_string("Cargo.toml")
        .expect("read Cargo.toml")
        .parse()
        .expect("parse Cargo.toml");

    let compat = manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("compat"))
        .expect("[package.metadata.compat] is missing from Cargo.toml");

    for key in ["minimum_client_version", "recommended_client_version"] {
        let value = compat
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{key} is missing from [package.metadata.compat]"));

        println!("cargo:rustc-env=HOODIK_{}={}", key.to_uppercase(), value);
    }
}
