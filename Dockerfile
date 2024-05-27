FROM postgres:alpine


ARG TARGETARCH
COPY ./eugene-${TARGETARCH} /usr/local/bin/eugene
RUN chmod +x /usr/local/bin/eugene

RUN adduser -D eugene
USER eugene

ENTRYPOINT ["/usr/local/bin/eugene"]
