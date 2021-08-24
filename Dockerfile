FROM rust:1.54-buster as stickyapp-rust-builder
WORKDIR /usr/src

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get install -y libfftw3-dev

# 1b: Download and compile Rust dependencies (and store as a separate Docker layer)
RUN USR=root cargo new stickyapp_rust
WORKDIR /usr/src/stickyapp_rust
COPY Cargo.toml Cargo.lock ./
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release && rm target/release/stickyapp_rust* target/release/deps/stickyapp_rust*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release  

# 2: Copy the exe and extra files ("static") to an empty Docker image
FROM debian:buster
RUN apt-get update && apt-get install -y apt-utils net-tools curl inetutils-ping libfftw3-dev && rm -rf /var/lib/apt/lists/*
COPY --from=stickyapp-rust-builder /usr/src/stickyapp_rust/target/release/stickyapp_rust .
CMD ["/stickyapp_rust"]
