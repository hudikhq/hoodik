use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
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
    let out_dir = PathBuf::from("src");
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

    let canonicalize_path = client_dist_dir.canonicalize().unwrap();
    let str_path = canonicalize_path.to_str().unwrap();

    writeln!(
        client_out_file,
        "pub(crate) const _DEFAULT: &[u8] = include_bytes!(concat!(\"{str_path}\", \"/index.html\"));"
    )?;

    writeln!(
        client_out_file,
        "pub(crate) const _CLIENT: [(&str, &[u8]); {}] = [",
        count_files(&client_dist_dir)?
    )?;

    let mut queue = vec![client_dist_dir.clone()];
    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                queue.push(path);
            } else {
                let relative_path = path
                    .strip_prefix(&client_dist_dir)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");

                writeln!(
                    client_out_file,
                    r#"("{relative_path}", include_bytes!(concat!("{str_path}", "/{relative_path}"))),"#,
                )?;
            }
        }
    }
    writeln!(client_out_file, "];")?;

    Ok(())
}

/// Count files in the directory
fn count_files(dir: &PathBuf) -> io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            count += count_files(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
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
