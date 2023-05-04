FROM ubuntu:latest

ENV HOST 0.0.0.0
EXPOSE 4554/tcp

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y

COPY ./target/release/hoodik /usr/local/bin

CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 4554

