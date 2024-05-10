FROM alpine

ARG TARGETARCH
COPY ./eugene-${TARGETARCH} /usr/local/bin/eugene
RUN chmod +x /usr/local/bin/eugene

ENTRYPOINT ["/usr/local/bin/eugene"]
