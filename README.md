# Hoodik

Introducing "Hoodik" - a lightweight, secure, and self-hosted cloud storage solution built in Rust and Vue. With end-to-end encryption, your data is protected from prying eyes and hackers. Hoodik supports file uploading and downloading, as well as file sharing with other users. With its simple and intuitive web interface, managing your files has never been easier. Plus, with Rust's focus on speed and performance, your data transfers will be lightning fast. Take control of your data with Hoodik.

<p align="center">
  <img src="./web/public/android-chrome-512x512.png" alt="Hoodik" />
</p>

# How does it work?

Hoodik is build to store your files encrypted without server knowledge of the encryption key. Files are encrypted and decrypted* in the client application**, when downloading and uploading.

To enable the end to end encryption to be as fast as possible with option to share file between application users the hybrid encryption approach is used:
 - User gets RSA keypair generated upon registration
 - Public key is stored with the user information on the server
 - Files are encrypted with the randomly generated AES key at the time of upload
 - File AES key is encrypted with users public key and is stored linking in the database encrypted linking the user and file
 - When sharing a file, you are effectively encrypting the file AES key with other users public key

Searching through the files without leaving the plaintext metadata in the database:
 - data that is considered searchable about the file (name, meta, etc..) is tokenized
 - resulting tokes are hashed and stored in the database as file tokens
 - search performs the same operation on the search query and sends it to the server
 - tokens are matched to the query and resulting files are fetched from the database

Publicly sharing links to files without leaking actual file AES key:
 - Random AES key for the link is generated
 - File metadata is encrypted with the link key
 - The original file AES key is encrypted with link key
 - Link key is encrypted with owners RSA key (so the owner can retrieve the key anytime)
 - When someone clicks the link it will either have the link key included in the link `https://.../links/{id}#link-key` or they will have to input it in the client app before starting the download
 - On the download request, link key is sent to the server where the actual file key is decrypted in-memory
 - File content is streamed for download while it is being decrypted in-memory

For RSA, 2048 [PKCS#1](https://en.wikipedia.org/wiki/PKCS_1) key is used, and for AES we are using [AEAD Ascon-128a](https://ascon.iaik.tugraz.at/).
Details of the crypto usage can be found in the `cryptfns` workspace member.
This crypto setup is used because it showed best performance results - so far.

Files are split and stored in chunks of max size defined in the `fs::MAX_CHUNK_SIZE_BYTES` constant. Each chunk is encrypted separately. The reason for storing files this way and encrypting them this way is to enable concurrent uploading and downloading of chunks to offset the encryption overhead.

**in case of downloading publicly linked files, the shared key is only to unlock the link, and within the link is encrypted actual file key which then decrypts the file as it is downloaded. This is implemented in this way so that the person receiving the shared link never gets the key to a file.*

***there is an option for encrypt and decrypt on the server, it can be used as a fallback option in case the client is running on a toaster and doesn't have enough compute power to encrypt/decrypt. But generally, this will probably never be used*



# Application state

The application is currently still in a pre alpha version, it doesn't even have proper tagging for releases. 

There are some issues that need to be worked out before the app is released in the alpha version:
 - Wasm crypto functions are fast, but not as I wish they were
 - Files cannot be shared
 - No public links can be made with files
 - Settings for the users account are non-existent
 - Overall database design is still in the early stages and might change without a migration (still dropping when upgrading)
 - Frontend application has lost testing ability once the app switched to credentials mode of authentication (Jest cannot deal with http only cookies)
 - Currently, its using AES-AEAD encryption for files, that might change and would be a breaking change
 - Files are stored on disk in chunks, that is currently working and it won't change most likely, but.. It might be necessary for performance
 - ...I don't feel like it being ready yet.

# Installing via docker

The application itself can handle incoming traffic, but for best results, use a reverse proxy, something like [Nginx Proxy Manager](https://nginxproxymanager.com/).

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
htunlogic/hoodik:latest
```

# Database

The application supports either `Sqlite` or `Postgres` databases, `Sqlite` is enabled by default and it creates a database file in your `DATA_DIR` so it works out of the box. But if you want, you can also use outside `Postgres` database, in that case, you will need to provide the `DATABASE_URL` for the `Postgres` connection.

# Configuration

For more detailed application configuration, please see [environment example](./.env.example)