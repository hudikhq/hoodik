ARG BIN_DIR=amd64

FROM alpine:3.18

# Inherit build variables
ARG BIN_DIR

# Install required runtime dependencies using apk
RUN apk add --no-cache bash libgcc openssl tini tzdata

COPY $BIN_DIR/hoodik /usr/local/bin

COPY package/docker/entrypoint.sh /opt/

EXPOSE 5443/tcp
ENV HOST 0.0.0.0
ENV DATA_DIR "/data"
ENV RUST_LOG "hoodik=debug,auth=debug,error=debug,entity=debug,storage=debug,context=debug,util=debug,cryptfns=debug,actix_web=debug"

# Use Tini to ensure that our application responds to CTRL-C when run in the
# foreground without the Docker argument "--init" (which is actually another
# way of activating Tini, but cannot be enabled from inside the Docker image).
ENTRYPOINT ["/sbin/tini", "--", "/opt/entrypoint.sh"]
CMD ["/usr/local/bin/hoodik", "-a", "0.0.0.0", "-p", "5443"]

