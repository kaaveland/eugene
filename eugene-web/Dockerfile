FROM ubuntu
COPY target/release/eugene-web /usr/local/bin/eugene-web
RUN chmod +x /usr/local/bin/eugene-web
EXPOSE 3000
CMD ["eugene-web"]
