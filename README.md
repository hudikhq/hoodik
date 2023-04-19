# Hoodik

Introducing "Hoodik" - a lightweight, secure, and self-hosted cloud storage solution built in Rust. With end-to-end encryption, your data is protected from prying eyes and hackers. Hoodik supports file uploading and downloading, as well as file sharing with other users. With its simple and intuitive web interface, managing your files has never been easier. Plus, with Rust's focus on speed and performance, your data transfers will be lightning fast. Take control of your data with Hoodik.

# Using through docker

```shell
docker run --name hoodik -it -d \
-e DATA_DIR='/data' \
--volume "$(pwd)/data:/data" \
-p 4554:4554 \
hoodik:latest
```

# License

This project (Hoodik) is created and maintained by Tibor Hudik and is licensed under the Creative Commons Attribution-NonCommercial 4.0 International (CC BY-NC 4.0) license.

You are free to:

- Share — copy and redistribute the material in any medium or format
- Adapt — remix, transform, and build upon the material

Under the following terms:

- Attribution — You must give appropriate credit, provide a link to the license, and indicate if changes were made. You may do so in any reasonable manner, but not in any way that suggests the licensor endorses you or your use.
- NonCommercial — You may not use the material for commercial purposes.

If you would like to use this project (Hoodik) for commercial purposes, please contact me at hello@hudik.eu to discuss a separate commercial license agreement.

For more information, please refer to the [CC BY-NC 4.0 license](https://creativecommons.org/licenses/by-nc/4.0/).
