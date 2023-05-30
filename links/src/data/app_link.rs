use chrono::NaiveDateTime;
use entity::{files, links, users, DbErr, FromQueryResult, QueryResult, Uuid};
use error::{AppResult, Error};
use fs::{prelude::Filename, IntoFilename};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppLink {
    pub id: Uuid,
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub owner_email: String,
    pub owner_pubkey: String,
    pub file_size: Option<i64>,
    pub file_mime: String,
    /// Signature that the user created when the link was initially created.
    ///
    /// The signature is made using a shared file_id
    pub signature: String,
    /// Number of times the file has been downloaded,
    /// this counts every attempt to download the file,
    /// it doesn't wait for the file to be downloaded in order to increment.
    pub downloads: i32,
    /// Name of the file encrypted with the link key. If the file is
    /// renamed after the link is created, that change won't be reflected
    /// in the link.
    pub encrypted_name: String,
    /// Link AES key encrypted with the user's public RSA key.
    /// So the owner of the link can retrieve the link key directly
    /// from the database entry, everyone else must have it shared with them.
    pub encrypted_link_key: String,
    /// If the file has a thumbnail it is encrypted with the link key.
    pub encrypted_thumbnail: Option<String>,
    /// AES key for the file encrypted with a link key.
    /// This property is not shared outside of the application.
    /// it is decrypted in memory before the file is downloaded.
    #[serde(skip_serializing)]
    pub encrypted_file_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub file_created_at: NaiveDateTime,
    /// Date when the link will expire, automated cron job
    /// will periodically empty out the expired links of all the
    /// file metadata and encrypted file key.
    pub expires_at: Option<NaiveDateTime>,
}

impl AppLink {
    /// Decrypt the link key with the AES link key
    pub fn decrypt_name(&self, link_key: &[u8]) -> AppResult<String> {
        let ciphertext = cryptfns::hex::decode(&self.encrypted_name)?;
        let plaintext = cryptfns::aes::decrypt(link_key.to_vec(), ciphertext)?;

        String::from_utf8(plaintext).map_err(Error::from)
    }

    /// Decrypt the file key with the AES link key
    pub fn file_key(&self, link_key: &[u8]) -> AppResult<Vec<u8>> {
        let file_key_hex = self
            .encrypted_file_key
            .as_ref()
            .ok_or_else(|| Error::from("app_link_has_no_file_key"))?;

        let file_key_ciphertext = cryptfns::hex::decode(file_key_hex)?;
        let file_key_plaintext = cryptfns::aes::decrypt(link_key.to_vec(), file_key_ciphertext)?;
        let string_hex_file_key = String::from_utf8(file_key_plaintext)?;
        let file_key = cryptfns::hex::decode(string_hex_file_key)?;

        Ok(file_key)
    }

    /// Let us know if the link has expired so we can prevent it from being downloaded.
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().naive_utc();

        self.expires_at
            .map(|expires_at| expires_at < now)
            .unwrap_or(false)
    }
}

impl FromQueryResult for AppLink {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let link = links::Model::from_query_result(res, "link")?;
        let user = users::Model::from_query_result(res, "user")?;
        let file = files::Model::from_query_result(res, "file")?;

        Ok(Self {
            id: link.id,
            file_id: file.id,
            file_size: file.size,
            file_mime: file.mime,
            signature: link.signature,
            downloads: link.downloads,
            encrypted_name: link.encrypted_name,
            encrypted_link_key: link.encrypted_link_key,
            encrypted_thumbnail: link.encrypted_thumbnail,
            encrypted_file_key: link.encrypted_file_key,
            created_at: link.created_at,
            file_created_at: file.created_at,
            expires_at: link.expires_at,
            owner_id: user.id,
            owner_email: user.email,
            owner_pubkey: user.pubkey,
        })
    }
}

impl IntoFilename for AppLink {
    fn filename(&self) -> AppResult<Filename> {
        Ok(Filename::new(self.file_created_at, self.file_id))
    }
}
