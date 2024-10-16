FROM clux/muslrust as build

WORKDIR /app/

COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev && apt-get clean
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine
WORKDIR /app/

COPY --from=build app/target/x86_64-unknown-linux-musl/release/loyalty-web ./


CMD ["./loyalty-web"]