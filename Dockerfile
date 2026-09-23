FROM rust:bookworm as builder

WORKDIR /prod
COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
# This is the trick to speed up the building process.
RUN cargo vendor > .cargo/config

COPY . .
RUN cargo build --release

# Use any runner as you want
# But beware that some images have old glibc which makes rust unhappy
FROM debian:bookworm-slim
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y openssl ca-certificates tzdata && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /prod/data/wordle_history.json /app/data/wordle_history.json
COPY --from=builder /prod/target/release/diffusion-bot-rust /bin
