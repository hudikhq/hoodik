FROM rust:latest

ENV NODE_VERSION=18

# Install dependencies
RUN apt-get update && \
  apt-get install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | bash

# Setup node
SHELL ["/bin/bash", "--login", "-c"]

RUN echo "export PATH=\"$PATH:/usr/local/cargo/bin\"" >> ~/.bashrc
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.35.3/install.sh | bash
RUN nvm install $NODE_VERSION && nvm use $NODE_VERSION && nvm alias default $NODE_VERSION

RUN node --version
RUN npm --version
RUN cargo --version