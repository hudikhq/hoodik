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
}

impl S3Config {
    pub(crate) fn new(vars: &mut Vars) -> Self {
        let bucket = vars.var_default::<String>("S3_BUCKET", "".to_string());
        let region = vars.var_default("S3_REGION", "us-east-1".to_string());
        let endpoint = vars.maybe_var::<String>("S3_ENDPOINT");
        let access_key = vars.var_default::<String>("S3_ACCESS_KEY", "".to_string());
        let secret_key = vars.var_default::<String>("S3_SECRET_KEY", "".to_string());
        let path_style = vars.var_default("S3_PATH_STYLE", false);
        let prefix = vars.maybe_var::<String>("S3_PREFIX");

        vars.panic_if_errors("S3Config");

        Self {
            bucket: bucket.get(),
            region: region.get(),
            endpoint: endpoint.maybe_get(),
            access_key: access_key.get(),
            secret_key: secret_key.get(),
            path_style: path_style.get(),
            prefix: prefix.maybe_get(),
        }
    }
}
