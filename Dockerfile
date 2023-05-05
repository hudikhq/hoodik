FROM ubuntu:latest

ENV HOST 0.0.0.0
EXPOSE 443/tcp

RUN apt update && apt install curl libpq-dev clang llvm pkg-config nettle-dev libc6-dev libssl-dev -y

COPY ./target/release/hoodik /usr/local/bin

ENV RUST_LOG="hoodik=debug,auth=debug,error=debug,entity=debug,storage=debug,context=debug,util=debug,cryptfns=debug"

# The application in the docker image will always run 
# with this address and port configuration, if you need 
# it to be on some different port, you can achieve that 
# with port mapping through the docker.
CMD /usr/local/bin/hoodik -a 0.0.0.0 -p 443

