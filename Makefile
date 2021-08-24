all: release
RUST_SOURCES=$(shell find src/ -type f)
release: $(RUST_SOURCES) Cargo.toml
	RUST_LOG="info" RUSTFLAGS="-C target-cpu=native" cargo run --release
debug: $(RUST_SOURCES) Cargo.toml
	RUST_LOG="info" RUSTFLAGS="-C target-cpu=native" cargo run
image: $(RUST_SOURCES) Cargo.toml
	docker build -t stickyapp_rust --build-arg HTTPS_PROXY=${HTTPS_PROXY} --build-arg HTTP_PROXY=${HTTP_PROXY} --build-arg http_proxy=${http_proxy} --build-arg https_proxy=${https_proxy} .

imagerun:
	docker run --rm -it -p 8080:8080 -e RUST_LOG=info stickyapp_rust
push:
	docker tag stickyapp_rust paarijaat/stickyapp_rust
	docker push paarijaat/stickyapp_rust
pushlocal:
	docker tag stickyapp_rust paarijaat-debian-vm:5000/paarijaat/stickyapp_rust
	docker push paarijaat-debian-vm:5000/paarijaat/stickyapp_rust
