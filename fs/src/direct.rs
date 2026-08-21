//! Whether this deployment can actually hand clients a URL to the bucket.
//!
//! Direct transfer only works when the store is reachable *from the client*,
//! and nothing about that is visible from inside the server: CORS is enforced
//! in the browser and never reported back, and a certificate the server trusts
//! may be one no phone has heard of. So an operator can switch the feature on
//! against a bucket that cannot serve it and see nothing wrong.
//!
//! These checks stand in for the client. When any fails the capability stays
//! off, the reason is logged, and every transfer keeps flowing through the
//! relaying routes, which never stopped working.

use config::direct::DirectVerdict;
use config::Config;
use std::net::IpAddr;

/// Object the probe addresses. Nothing reads or writes it — a CORS preflight
/// is answered from the bucket's policy whether or not the key exists.
const PROBE_KEY: &str = ".hoodik-direct-probe";

/// Run the checks once and record the answer for [`config::direct::verdict`].
/// Called during startup, before the server binds.
///
/// Deliberately not re-run per request: fixing a bucket's CORS policy takes a
/// restart to take effect here, which is a better trade than a network round
/// trip on the way to answering what the server can do.
///
/// Never fails. An unreachable bucket is a reason to leave direct transfer
/// off, not a reason to refuse to boot.
pub async fn probe(config: &Config) {
    let verdict = evaluate(config).await;

    if verdict.enabled {
        log::info!("Direct S3 transfer enabled: clients will read and write chunks from the bucket");
    } else if !verdict.blockers.is_empty() {
        log::warn!(
            "S3_DIRECT_TRANSFER is set but direct transfer stays off: {}. \
             Transfers continue through this server.",
            verdict.blockers.join("; ")
        );
    }

    config::direct::record(verdict);
}

async fn evaluate(config: &Config) -> DirectVerdict {
    if config.app.storage_provider != "s3" {
        return DirectVerdict::default();
    }

    #[cfg(not(feature = "s3"))]
    {
        let _ = config;
        DirectVerdict::default()
    }

    #[cfg(feature = "s3")]
    {
        let Some(s3) = config.s3.as_ref() else {
            return DirectVerdict::default();
        };

        if !s3.direct_transfer {
            return DirectVerdict::default();
        }

        let mut blockers = Vec::new();

        let probe_url = match probe_url(s3).await {
            Ok(url) => url,
            Err(e) => {
                return DirectVerdict {
                    enabled: false,
                    blockers: vec![format!("could not derive a bucket URL to check ({e})")],
                }
            }
        };

        if !s3.direct_allow_insecure {
            blockers.extend(transport_blockers(&probe_url));
        }

        // The preflight doubles as the certificate check: this client trusts
        // the public roots only, so an endpoint whose certificate is signed by
        // an internal CA fails here exactly as it would in a browser, even
        // though the server's own S3 calls accept it. The insecure flag
        // waives exactly that part — the CORS questions still have to be
        // answered correctly.
        blockers.extend(
            cors_blockers(
                &probe_url,
                config.get_app_url().as_str(),
                s3.direct_allow_insecure,
            )
            .await,
        );

        DirectVerdict {
            enabled: blockers.is_empty(),
            blockers,
        }
    }
}

/// Reasons the transport would fail a browser, independent of CORS.
#[cfg(feature = "s3")]
fn transport_blockers(url: &url::Url) -> Vec<String> {
    let mut blockers = Vec::new();

    if url.scheme() != "https" {
        blockers.push(format!(
            "the endpoint is {}, and a page served over HTTPS cannot fetch \
             plain HTTP — unlike a bad certificate there is no way for the \
             user to override it",
            url.scheme()
        ));
    }

    if let Some(host) = url.host_str() {
        if is_unreachable_from_clients(host) {
            blockers.push(format!(
                "'{host}' resolves only on the server's own network, so a \
                 signed URL would point somewhere the client cannot reach. \
                 Set S3_DIRECT_ALLOW_INSECURE if every client is on that \
                 network too"
            ));
        }
    }

    blockers
}

/// Hosts that a client elsewhere on the internet has no way to resolve or
/// route to. A bare name with no dot is the `http://minio:9000` container
/// case, which is what the project's own MinIO example has always used.
#[cfg(feature = "s3")]
fn is_unreachable_from_clients(host: &str) -> bool {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
        };
    }

    let host = host.to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".localhost")
        || host.ends_with(".internal")
        || !host.contains('.')
}

/// Ask the bucket the same question a browser asks before it will let a page
/// read a cross-origin response, once per method the feature needs.
#[cfg(feature = "s3")]
async fn cors_blockers(url: &url::Url, origin: &str, allow_insecure: bool) -> Vec<String> {
    // `use_rustls_tls` is what makes this a browser stand-in rather than
    // another server-side call: it pins the backend to rustls, whose root
    // store here is the public webpki set. reqwest's default native-tls
    // backend would read the host's trust store and happily accept the
    // internal CA that no client of ours has.
    let mut builder = reqwest::Client::builder().use_rustls_tls();

    // S3_DIRECT_ALLOW_INSECURE exists for the home-lab bucket whose
    // certificate only the operator's own devices trust. The certificate
    // check lives inside this preflight, so honouring the flag means
    // accepting whatever certificate the endpoint presents here.
    if allow_insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }

    let client = match builder.build() {
        Ok(client) => client,
        Err(e) => return vec![format!("could not build an HTTP client to check CORS ({e})")],
    };

    let mut blockers = Vec::new();
    for method in ["GET", "PUT"] {
        if let Err(reason) = preflight(&client, url, origin, method).await {
            blockers.push(reason);
        }
    }
    blockers
}

#[cfg(feature = "s3")]
async fn preflight(
    client: &reqwest::Client,
    url: &url::Url,
    origin: &str,
    method: &str,
) -> Result<(), String> {
    let response = client
        .request(reqwest::Method::OPTIONS, url.as_str())
        .header("Origin", origin)
        .header("Access-Control-Request-Method", method)
        .send()
        .await
        .map_err(|e| {
            format!("the bucket did not answer a {method} CORS preflight from {origin} ({e})")
        })?;

    let allowed_origin = header(&response, "access-control-allow-origin").ok_or_else(|| {
        format!(
            "the bucket answered a {method} preflight without \
             Access-Control-Allow-Origin, so a browser would discard the \
             response. Add a CORS rule allowing {origin}"
        )
    })?;

    if !origin_allowed(&allowed_origin, origin) {
        return Err(format!(
            "the bucket's CORS rule allows '{allowed_origin}' but this instance is {origin}"
        ));
    }

    if !method_allowed(header(&response, "access-control-allow-methods").as_deref(), method) {
        return Err(format!(
            "the bucket's CORS rule does not allow {method} from {origin}"
        ));
    }

    Ok(())
}

/// A wildcard is legal here because these requests carry no credentials — a
/// credentialed one would oblige the bucket to name the origin exactly, which
/// is why the direct path sends none.
#[cfg(feature = "s3")]
fn origin_allowed(allowed: &str, origin: &str) -> bool {
    allowed == "*" || allowed.trim_end_matches('/') == origin.trim_end_matches('/')
}

/// Some backends echo back only the method that was asked about, others list
/// everything they permit. Either answer names the method when it is allowed,
/// and an absent header means the backend did not restrict methods at all.
#[cfg(feature = "s3")]
fn method_allowed(allowed: Option<&str>, method: &str) -> bool {
    match allowed {
        None => true,
        Some("*") => true,
        Some(list) => list
            .split(',')
            .any(|candidate| candidate.trim().eq_ignore_ascii_case(method)),
    }
}

#[cfg(feature = "s3")]
fn header(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
}

/// The URL a client would actually be sent to, taken by signing the probe key
/// and dropping the query. Derived rather than assembled so the check covers
/// whatever host and path shape the real signing produces, path-style
/// addressing included.
#[cfg(feature = "s3")]
async fn probe_url(s3: &config::s3::S3Config) -> Result<url::Url, String> {
    let provider = crate::providers::s3::S3Provider::new(s3);
    let key = format!("{}{}", provider.prefix(), PROBE_KEY);

    let signed = provider.presign_get(&key).await.map_err(|e| e.to_string())?;

    let mut url = url::Url::parse(&signed).map_err(|e| e.to_string())?;
    url.set_query(None);

    Ok(url)
}

#[cfg(all(test, feature = "s3"))]
mod test {
    use super::{is_unreachable_from_clients, method_allowed, origin_allowed, transport_blockers};

    fn blockers(url: &str) -> Vec<String> {
        transport_blockers(&url::Url::parse(url).unwrap())
    }

    /// A page served over HTTPS cannot fetch plain HTTP, and there is no
    /// interstitial for a subresource the way there is for a bad certificate.
    /// So an `http://` endpoint is refused rather than warned about.
    #[test]
    fn plain_http_endpoints_are_blocked() {
        let reasons = blockers("http://files.example.org/bucket/key");
        assert_eq!(reasons.len(), 1, "expected only the scheme to be faulted: {reasons:?}");
        assert!(reasons[0].contains("HTTPS"), "{}", reasons[0]);
    }

    #[test]
    fn https_public_endpoints_have_no_blockers() {
        assert!(blockers("https://abc.r2.cloudflarestorage.com/bucket/key").is_empty());
    }

    /// The documented MinIO setup: plain HTTP on a container name. Both
    /// problems are reported, because fixing one still leaves it unusable.
    #[test]
    fn the_documented_minio_example_is_blocked_twice() {
        let reasons = blockers("http://minio:9000/hoodik/key");
        assert_eq!(reasons.len(), 2, "{reasons:?}");
    }

    #[test]
    fn origin_matching_accepts_a_wildcard_and_ignores_a_trailing_slash() {
        assert!(origin_allowed("*", "https://drive.example.org"));
        assert!(origin_allowed("https://drive.example.org", "https://drive.example.org"));
        assert!(origin_allowed("https://drive.example.org/", "https://drive.example.org"));
        assert!(origin_allowed("https://drive.example.org", "https://drive.example.org/"));
    }

    #[test]
    fn origin_matching_rejects_a_different_origin() {
        assert!(!origin_allowed("https://other.example.org", "https://drive.example.org"));
        assert!(!origin_allowed("", "https://drive.example.org"));
    }

    #[test]
    fn method_matching_reads_a_list_or_a_single_echo() {
        assert!(method_allowed(Some("GET, PUT, HEAD"), "PUT"));
        assert!(method_allowed(Some("GET"), "GET"));
        assert!(method_allowed(Some("*"), "PUT"));
        assert!(method_allowed(Some("get, put"), "PUT"));
        // Absent means the backend did not restrict methods.
        assert!(method_allowed(None, "PUT"));
    }

    #[test]
    fn method_matching_rejects_a_method_that_is_not_listed() {
        assert!(!method_allowed(Some("GET, HEAD"), "PUT"));
        assert!(!method_allowed(Some(""), "GET"));
    }

    /// Substring matching would accept a method that merely appears inside
    /// another token, which is how a read-only bucket ends up advertised as
    /// writable.
    #[test]
    fn method_matching_does_not_match_a_substring() {
        assert!(!method_allowed(Some("TARGET"), "GET"));
        assert!(!method_allowed(Some("INPUT"), "PUT"));
    }

    #[test]
    fn private_and_container_hosts_are_unreachable() {
        for host in [
            "localhost",
            "minio",
            "127.0.0.1",
            "10.0.0.5",
            "192.168.1.20",
            "172.16.3.4",
            "nas.local",
            "bucket.internal",
            "::1",
        ] {
            assert!(is_unreachable_from_clients(host), "{host} should be rejected");
        }
    }

    #[test]
    fn public_hosts_are_reachable() {
        for host in [
            "abc123.r2.cloudflarestorage.com",
            "s3.eu-west-2.amazonaws.com",
            "files.example.org",
            "8.8.8.8",
        ] {
            assert!(!is_unreachable_from_clients(host), "{host} should be accepted");
        }
    }
}
