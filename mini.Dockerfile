FROM debian:bullseye-slim

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y
ENV HOST 0.0.0.0
EXPOSE 4554/tcp


COPY ./target/release/hoodik /usr/local/bin

CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 4554

