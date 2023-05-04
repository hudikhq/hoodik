# Hoodik

Introducing "Hoodik" - a lightweight, secure, and self-hosted cloud storage solution built in Rust. With end-to-end encryption, your data is protected from prying eyes and hackers. Hoodik supports file uploading and downloading, as well as file sharing with other users. With its simple and intuitive web interface, managing your files has never been easier. Plus, with Rust's focus on speed and performance, your data transfers will be lightning fast. Take control of your data with Hoodik.

# Using through docker

```shell
docker run --name hoodik -it -d \
-e DATA_DIR='/data' \
--volume "$(pwd)/data:/data" \
-p 4554:4554 \
htunlogic/hoodik:latest
```
