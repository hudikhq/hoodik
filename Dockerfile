FROM rust:latest AS builder

RUN curl -sL https://deb.nodesource.com/setup_18.x | bash
RUN apt-get update && apt-get install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev nodejs npm -y
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | bash

WORKDIR /home/circleci
COPY . /home/circleci

RUN npm install -g yarn
RUN yarn install
RUN yarn release:all

FROM debian:bullseye-slim

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y
ENV HOST 0.0.0.0
EXPOSE 4554/tcp

RUN useradd rust
USER rust:rust
COPY --from=builder \
  /home/circleci/target/release/hoodik \
  /usr/local/bin

CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 4554

