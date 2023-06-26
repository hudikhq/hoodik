use url::Url;

pub fn generate<T: ToString>(maybe_url: T) -> Option<Url> {
    let url = maybe_url.to_string();

    Url::parse(&url).ok()
}
