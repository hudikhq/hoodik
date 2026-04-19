use path_absolutize::Absolutize;
use std::path;

pub(crate) fn absolute_path(path: &str) -> Option<String> {
    let p = path::Path::new(path);

    Some(p.absolutize().ok()?.to_string_lossy().to_string())
}

pub(crate) fn remove_trailing_slash(path: String) -> String {
    let mut path = path.trim().to_string();

    if path.ends_with('/') {
        let _ = path.pop();
    }

    path
}
