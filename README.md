# HOODIK
Hoodik is a lightweight, secure, and self-hosted cloud storage solution that we've built using Rust and Vue. With end-to-end encryption, your data is shielded from prying eyes and hackers. Hoodik supports file uploading and downloading - making it easy for you to share files with other users. Our web interface is simple and intuitive, making file management a breeze. Plus, thanks to Rust's focus on speed and performance, your data transfers will validate the meaning of 'lightning fast'. 

<p align="center">
  <img src="./web/public/android-chrome-512x512.png" alt="Hoodik" />
</p>

# Features

We designed Hoodik with a central goal: to store your files securely, despite the server not having access to your encryption key. Your files are encrypted and decrypted in the client application during downloading and uploading.

To ensure end-to-end encryption is as fast as possible and to enable file sharing among application users, we chose a hybrid encryption approach:
- Upon registration, each user receives a generated RSA key pair.
- We store your public key with your information on the server.
- We encrypt files with a randomly generated AES key during upload.
- We encrypt the file's AES key with the user's public key and store it in the database, thus linking the user and the file.

To enable you to search through your files without leaving plaintext metadata in the database, we've set the following mechanism in place:
- We tokenize any data about the file considered searchable (name, metadata, etc.).
- We hash the resulting tokens and store them in the database as file tokens.
- When you perform a search, we perform the same operation on your search query and transmit it to the server.
- The server matches tokens to the query, and fetches corresponding files from the database.

We created a process for publicly sharing links to files that doesn't leak the actual file's AES key:
- We generate a random AES key for the link.
- We encrypt the file metadata with the link key.
- We encrypt the original file's AES key with the link key.
- We encrypt the link key with the owner's RSA key (enabling the owner to retrieve the key anytime).
- When someone clicks the link, the link key will either be included in the link `https://.../links/{id}#link-key`, or the user will have to input it in the client app before starting the download.
- On the download request, the link key is sent to the server where the actual file key is decrypted in-memory.
- The file content is streamed for download while being decrypted in-memory.

For RSA, we employ 2048-bit [PKCS#1](https://en.wikipedia.org/wiki/PKCS_1) keys, and for AES, we use [AEAD Ascon-128a](https://ascon.iaik.tugraz.at/). We have detailed usage of the crypto in the `cryptfns` workspace member. Our encryption setup offers the best performance results we’ve seen to date.

We store files in chunks (defined by the `fs::MAX_CHUNK_SIZE_BYTES` constant) and encrypt each chunk individually. This method enables concurrent uploading and downloading of chunks, thereby offsetting encryption overhead.

*In the case of downloading publicly linked files, the shared key solely unlocks the link. Within the link, we've encrypted the actual file key, which then decrypts the file as it downloads. This design ensures that the person receiving the shared link never receives the file key.

**We offer the option of server-based encryption and decryption, which could be a fallback solution if the client is running on a device with limited computing power. However, we anticipate that this feature will rarely need to be used.*

# Installing via Docker

While the application itself can handle incoming traffic, we recommend using a reverse proxy, such as [Nginx Proxy Manager](https://nginxproxymanager.com/), for optimal results.

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

Our application supports either `Sqlite` or `Postgres` databases. `Sqlite` is enabled by default and will create a database file in your `DATA_DIR`, so it functions right out of the box. If you prefer, you can also use an external `Postgres` database - you'd just need to supply the `DATABASE_URL` for your `Postgres` connection.

# Configuration

For a more detailed application configuration, please see our [environment example](./.env.example).
