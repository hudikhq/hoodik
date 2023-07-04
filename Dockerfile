FROM ubuntu:latest

ENV HOST 0.0.0.0
EXPOSE 5443

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y

COPY ./target/release/hoodik /usr/local/bin

ENV DATA_DIR="/data"
ENV RUST_LOG="hoodik=debug,auth=debug,error=debug,entity=debug,storage=debug,context=debug,util=debug,cryptfns=debug,actix_web=debug"

CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 5443

