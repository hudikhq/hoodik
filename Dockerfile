FROM rustlang/rust:nightly-bullseye AS builder
ADD --chown=rust:rust . ./

WORKDIR /home/rust/app

RUN apt update && apt install clang llvm pkg-config nettle-dev -y
RUN cargo build --out-dir=./target -Z unstable-options --release

FROM debian:bullseye-slim

RUN apt update && apt install curl libpq-dev -y
ENV HOST 0.0.0.0
EXPOSE 4554/tcp

RUN useradd rust
USER rust:rust
COPY --from=builder \
  /home/rust/app/target/hoodik \
  /usr/local/bin

CMD /usr/local/bin/hoodik