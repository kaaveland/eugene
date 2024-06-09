FROM postgres:alpine

ARG TARGETARCH
RUN apk add git
COPY ./eugene-${TARGETARCH} /usr/local/bin/eugene
RUN chmod +x /usr/local/bin/eugene

RUN adduser -D eugene
USER eugene


ENTRYPOINT ["/usr/local/bin/eugene"]
