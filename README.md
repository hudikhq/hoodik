# HOODIK

A lightweight, secure, and self-hosted cloud storage solution built in Rust and Vue. With end-to-end encryption, your data is protected from prying eyes and hackers. Hoodik supports file uploading and downloading, as well as file sharing with other users. Its simple and intuitive web interface makes managing your files easier than ever. Plus, with Rust's focus on speed and performance, your data transfers will be lightning fast. 

<p align="center">
  <img src="./web/public/android-chrome-512x512.png" alt="Hoodik" />
</p>

# Features

Hoodik is built to store your files encrypted, without the server having knowledge of the encryption key. Files are encrypted and decrypted* in the client application** during downloading and uploading.

To enable end-to-end encryption to be as fast as possible, with the option to share files between application users, the hybrid encryption approach is used:
- Upon registration, the user gets a generated RSA keypair.
- The public key is stored with the user's information on the server.
- Files are encrypted with a randomly generated AES key at the time of upload.
- The file's AES key is encrypted with the user's public key and stored in the database, linking the user and file.

Searching through the files without leaving plaintext metadata in the database works as follows:
- Data considered searchable about the file (name, metadata, etc.) is tokenized.
- The resulting tokens are hashed and stored in the database as file tokens.
- During a search, the same operation is performed on the search query and sent to the server.
- Tokens are matched to the query, and resulting files are fetched from the database.

Publicly sharing links to files without leaking the actual file's AES key is accomplished through the following steps:
- A random AES key is generated for the link.
- File metadata is encrypted with the link key.
- The original file's AES key is encrypted with the link key.
- The link key is encrypted with the owner's RSA key (so the owner can retrieve the key anytime).
- When someone clicks the link, the link key will either be included in the link `https://.../links/{id}#link-key`, or they will have to input it in the client app before starting the download.
- On the download request, the link key is sent to the server, where the actual file key is decrypted in-memory.
- The file content is streamed for download while it is being decrypted in-memory.

For RSA, we use 2048-bit [PKCS#1](https://en.wikipedia.org/wiki/PKCS_1) keys, and for AES, we use [AEAD Ascon-128a](https://ascon.iaik.tugraz.at/).
Details of the crypto usage can be found in the `cryptfns` workspace member.
This crypto setup is used because it has shown the best performance results so far.

Files are split and stored in chunks of a maximum size defined in the `fs::MAX_CHUNK_SIZE_BYTES` constant. Each chunk is encrypted separately. Storing files in this way and encrypting them enables concurrent uploading and downloading of chunks to offset the encryption overhead.

**In the case of downloading publicly linked files, the shared key is only used to unlock the link. Within the link, the actual file key is encrypted, which then decrypts the file as it is downloaded. This implementation ensures that the person receiving the shared link never gets the file key.*

***There is an option for encrypting and decrypting on the server. It can be used as a fallback option in case the client is running on a device with limited computing power. However, it is unlikely to be used in general.*

# Installing via Docker

The application itself can handle incoming traffic, but for best results, use a reverse proxy, such as [Nginx Proxy Manager](https://nginxproxymanager.com/).

```shell
docker run --name hoodik -it -d \
-e DATA_DIR='/data' \
-e APP_URL='https://my-app.local' \
-e SSL_CERT_FILE='/data/my-cert-file.crt.pem' \
-e SSL_KEY_FILE='/data/my-key-file.key.pem' \
-e MAILER_TYPE='smtp' \
-e SMTP_ADDRESS='smtp.gmail.com' \
-e SMTP_USERNAME='email@gmail.com' \
-e SMTP_PASSWORD='google-account-app-password' \
-e SMTP_PORT='465' \
-e SMTP_DEFAULT_FROM='Hoodik Drive <email@gmail.com>' \
--volume "$(pwd)/data:/data" \
-p 4554:5443 \
hudik/hoodik:latest
```

# Database

The application supports either `Sqlite` or `Postgres` databases. `Sqlite` is enabled by default, and it creates a database file in your `DATA_DIR`, so it works out of the box. If desired, you can also use an external `Postgres` database. In that case, you will need to provide the `DATABASE_URL` for the `Postgres` connection.

# Configuration

For more detailed application configuration, please see the [environment example](./.env.example).