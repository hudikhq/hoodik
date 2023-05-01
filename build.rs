use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

/// If no dist dir is available, we will just write an empty client
fn handle_no_dist(client_out_file: &mut File) -> io::Result<()> {
    writeln!(client_out_file, "pub const _DEFAULT: &'static [u8] = &[];",)?;

    writeln!(
        client_out_file,
        "pub const _CLIENT: [(&'static str, &'static [u8]); 0] = [];",
    )?;

    Ok(())
}

fn main() -> io::Result<()> {
    let client_dist_dir = PathBuf::from("clients/web/dist");
    let out_dir = PathBuf::from("src");
    let mut client_out_file = File::create(out_dir.join("client.rs"))?;

    if !client_dist_dir.exists() {
        return handle_no_dist(&mut client_out_file);
    }

    writeln!(
        client_out_file,
        "pub const _DEFAULT: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/clients/web/dist/index.html\"));",
    )?;

    writeln!(
        client_out_file,
        "pub const _CLIENT: [(&str, &[u8]); {}] = [",
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
                    r#"("{}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/clients/web/dist/{}"))),"#,
                    relative_path, relative_path
                )?;
            }
        }
    }
    writeln!(client_out_file, "];")?;

    println!("cargo:rerun-if-changed=clients/web/dist");

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
