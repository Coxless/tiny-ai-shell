.PHONY: build build-static install clean test

build:
	cargo build --release
	mkdir -p bin
	cp target/release/ta bin/ta

build-static:
	cargo build --release --target x86_64-unknown-linux-musl
	mkdir -p bin
	cp target/x86_64-unknown-linux-musl/release/ta bin/ta

install: build
	sudo cp bin/ta /usr/local/bin/ta

clean:
	cargo clean
	rm -rf bin

test:
	cargo test
