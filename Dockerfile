FROM htunlogic/hoodik-builder:latest AS builder

WORKDIR /home/app
COPY . /home/app

# RUN cargo build --release
RUN npm install -g yarn
RUN yarn install
RUN yarn wasm-pack
RUN yarn release:all

FROM debian:bullseye-slim

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y
ENV HOST 0.0.0.0
EXPOSE 4554/tcp

RUN useradd rust
USER rust:rust
COPY --from=builder \
  /home/app/target/release/hoodik \
  /usr/local/bin

CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 4554

