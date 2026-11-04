FROM rust:1-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY sim ./sim
COPY net ./net
# Cargo resolves the workspace; the server does not compile or link the game.
COPY game/Cargo.toml ./game/Cargo.toml
RUN mkdir -p game/src && printf 'fn main() {}\n' > game/src/main.rs
RUN cargo build --release --locked -p racecraft-net --bin racecraft-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 racecraft
WORKDIR /app
COPY --from=build /build/target/release/racecraft-server ./racecraft-server
COPY deploy/render/assets ./assets
COPY deploy/render/rules.json ./rules.json
USER racecraft
ENV PORT=10000
EXPOSE 10000
CMD ["./racecraft-server", "--websocket", "--assets", "./assets", "--settings", "./rules.json"]
