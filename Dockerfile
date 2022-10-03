FROM alpine

WORKDIR /app
ENV LOG_LEVEL=info
ENTRYPOINT ["./lighting-manager"]

COPY target/arm-unknown-linux-gnueabihf/release/lighting-manager /app/
RUN chmod +x /app/lighting-manager
