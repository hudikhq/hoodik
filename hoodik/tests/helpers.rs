pub const CHUNKS: usize = 5;
pub const CHUNK_SIZE_BYTES: i32 = 1024 * 1024;

/// Helper for testing to extract the cookies
#[allow(dead_code)]
pub fn extract_cookies(
    headers: &actix_web::http::header::HeaderMap,
) -> (
    Option<actix_web::cookie::Cookie<'static>>,
    Option<actix_web::cookie::Cookie<'static>>,
) {
    let mut cookies = headers
        .get_all("set-cookie")
        .clone()
        .map(|h| {
            let h = h.clone().to_str().unwrap().to_string();

            actix_web::cookie::Cookie::parse(h).unwrap()
        })
        .collect::<Vec<actix_web::cookie::Cookie<'static>>>()
        .into_iter();

    let jwt = cookies.clone().find(|c| c.name() == "hoodik_session");
    let refresh = cookies.find(|c| c.name() == "hoodik_refresh");

    (jwt, refresh)
}

/// Helper to create some mock file for uploading
#[allow(dead_code)]
pub fn create_byte_chunks() -> (Vec<Vec<u8>>, i64, String) {
    let one_chunk_size = CHUNK_SIZE_BYTES as usize;
    let mut byte_chunks = vec![];
    let mut body = vec![];

    while body.len() < (one_chunk_size * CHUNKS) {
        body.extend(b"a");
    }

    let checksum = cryptfns::sha256::digest(body.as_slice());

    for i in (0..body.len()).step_by(one_chunk_size) {
        let chunk = &body[i..(i + one_chunk_size)];
        byte_chunks.push(chunk.to_vec());
    }

    let total_len = byte_chunks.iter().map(|chunk| chunk.len()).sum::<usize>() as i64;

    (byte_chunks, total_len, checksum)
}
