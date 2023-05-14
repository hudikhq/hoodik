# Hoodik

Introducing "Hoodik" - a lightweight, secure, and self-hosted cloud storage solution built in Rust and Vue. With end-to-end encryption, your data is protected from prying eyes and hackers. Hoodik supports file uploading and downloading, as well as file sharing with other users. With its simple and intuitive web interface, managing your files has never been easier. Plus, with Rust's focus on speed and performance, your data transfers will be lightning fast. Take control of your data with Hoodik.

<p align="center">
  <img src="./web/public/android-chrome-512x512.png" alt="Hoodik" />
</p>

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