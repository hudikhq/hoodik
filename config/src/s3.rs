use crate::vars::Vars;

#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3_BUCKET where file chunks will be stored
    ///
    /// *required*
    pub bucket: String,

    /// S3_REGION
    ///
    /// *optional*
    ///
    /// default: us-east-1
    pub region: String,

    /// S3_ENDPOINT for S3-compatible services (MinIO, Backblaze B2, Wasabi, etc.)
    ///
    /// *optional*
    pub endpoint: Option<String>,

    /// S3_ACCESS_KEY
    ///
    /// *required*
    pub access_key: String,

    /// S3_SECRET_KEY
    ///
    /// *required*
    pub secret_key: String,

    /// S3_PATH_STYLE use path-style addressing (required for MinIO)
    ///
    /// *optional*
    ///
    /// default: false
    pub path_style: bool,

    /// S3_PREFIX optional key prefix for all objects
    ///
    /// *optional*
    pub prefix: Option<String>,

    /// S3_DIRECT_TRANSFER lets clients read and write chunks straight from
    /// the bucket over presigned URLs, so ciphertext stops passing through
    /// this server in either direction.
    ///
    /// *optional*
    ///
    /// default: false
    ///
    /// Off by default because it only works when the bucket is reachable
    /// *from the client*, over HTTPS, with a publicly-trusted certificate
    /// and a CORS policy naming this instance. A bucket that only the
    /// server can reach — the usual `http://minio:9000` container setup —
    /// cannot serve it. The checks in the `fs` crate verify all of that at
    /// readiness and leave the capability switched off when any fails, so
    /// a wrong setting here costs a log line rather than a broken client.
    pub direct_transfer: bool,

    /// S3_DIRECT_EXPIRY_SECS how long a presigned chunk URL stays valid.
    ///
    /// *optional*
    ///
    /// default: 604800 (7 days, the SigV4 ceiling)
    ///
    /// Every URL for a transfer is signed up front so the whole set can go
    /// to the OS download queue in one go and survive the app being
    /// suspended for days. That makes the signature outlive the transfer by
    /// necessity, and the default matches the longest SigV4 permits. The
    /// exposure it buys is small: a URL is a bearer token for one chunk of
    /// ciphertext under a random key, useless without the file key, which
    /// never leaves the client.
    pub direct_expiry_secs: u32,

    /// S3_DIRECT_ALLOW_INSECURE skips the transport preconditions for
    /// direct transfer: the HTTPS requirement, the certificate check, and
    /// the private-address check.
    ///
    /// *optional*
    ///
    /// default: false
    ///
    /// For deployments where the clients sit on the same network as the
    /// bucket — a home NAS serving only LAN devices — and for the test
    /// harness, which runs everything over plain HTTP on localhost. CORS is
    /// still required: browsers enforce it whatever the network looks like.
    pub direct_allow_insecure: bool,
}

/// SigV4 refuses to sign a URL that outlives this, and so does `rust-s3`.
pub const MAX_PRESIGN_EXPIRY_SECS: u32 = 604_800;

impl S3Config {
    pub(crate) fn new(vars: &mut Vars) -> Self {
        let bucket = vars.var_default::<String>("S3_BUCKET", "".to_string());
        let region = vars.var_default("S3_REGION", "us-east-1".to_string());
        let endpoint = vars.maybe_var::<String>("S3_ENDPOINT");
        let access_key = vars.var_default::<String>("S3_ACCESS_KEY", "".to_string());
        let secret_key = vars.var_default::<String>("S3_SECRET_KEY", "".to_string());
        let path_style = vars.var_default("S3_PATH_STYLE", false);
        let prefix = vars.maybe_var::<String>("S3_PREFIX");
        let direct_transfer = vars.var_default("S3_DIRECT_TRANSFER", false);
        let direct_expiry_secs =
            vars.var_default("S3_DIRECT_EXPIRY_SECS", MAX_PRESIGN_EXPIRY_SECS);
        let direct_allow_insecure = vars.var_default("S3_DIRECT_ALLOW_INSECURE", false);

        vars.panic_if_errors("S3Config");

        let direct_expiry_secs = direct_expiry_secs.get();

        // Catch an over-long expiry here rather than at the first presign,
        // where it would surface as a failed download on a server that
        // started up clean.
        if direct_expiry_secs > MAX_PRESIGN_EXPIRY_SECS {
            panic!(
                "S3_DIRECT_EXPIRY_SECS is {direct_expiry_secs}, above the \
                 SigV4 maximum of {MAX_PRESIGN_EXPIRY_SECS}."
            );
        }

        Self {
            bucket: bucket.get(),
            region: region.get(),
            endpoint: endpoint.maybe_get(),
            access_key: access_key.get(),
            secret_key: secret_key.get(),
            path_style: path_style.get(),
            prefix: prefix.maybe_get(),
            direct_transfer: direct_transfer.get(),
            direct_expiry_secs,
            direct_allow_insecure: direct_allow_insecure.get(),
        }
    }
}
