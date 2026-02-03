FROM rust:1.89 AS base

WORKDIR /app
RUN cargo install sqlx-cli --no-default-features --features rustls,postgres
COPY Cargo.toml .
COPY src ./src
COPY migrations ./migrations
RUN --mount=type=secret,id=db,env=DATABASE_URL cargo build --release
COPY .env .env
EXPOSE 6942
CMD ["./target/release/backend"]
