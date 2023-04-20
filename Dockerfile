FROM rust AS builder

WORKDIR /usr/src/app

COPY . .

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev -y
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev -y
ENV HOST 0.0.0.0
EXPOSE 4554/tcp

RUN useradd rust
USER rust:rust
COPY --from=builder \
  /usr/src/app/release/hoodik \
  /usr/local/bin

CMD /usr/local/bin/hoodik