FROM debian:stable-slim

ARG TARGETARCH
COPY ./eugene-${TARGETARCH} /usr/bin/local/eugene
RUN chmod +x /usr/local/bin/eugene

ENTRYPOINT ["/usr/local/bin/eugene"]
