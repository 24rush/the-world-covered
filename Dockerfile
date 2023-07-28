# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM rust:1.69-slim
RUN apt-get update && apt-get install -y build-essential checkinstall zlib1g-dev -y

# Copy local code to the container image.
WORKDIR .
COPY . .

EXPOSE 8080:8080

RUN cargo install --path .
CMD ["local_server"]