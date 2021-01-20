FROM rust:1.49.0-alpine3.12

RUN apk add --update \
    bash ca-certificates openssl \
    nano libressl-dev

ENV DOCKERIZE_VERSION v0.6.1
RUN wget https://github.com/jwilder/dockerize/releases/download/$DOCKERIZE_VERSION/dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && tar -C /usr/local/bin -xzvf dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && rm dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz

COPY ./app /usr/src/app
COPY docker/ /files
RUN cp -rf /files/* /
RUN rm -rf /files
RUN cargo build --release

WORKDIR /usr/src/app
ENTRYPOINT ["dockerize", "-template", "/env.tmpl:/usr/src/app/.env"]
CMD ["bash", "/start.sh"]