FROM rust:1.73.0-buster
WORKDIR /usr/src/list
COPY backend .
ENV SQLX_OFFLINE=true
RUN cargo build --release
RUN cargo install --path .
CMD ["backend"]
