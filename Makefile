debug:
	RUSTFLAGS=-Awarnings RUST_BACKTRACE=1 cargo run --features watch

test:
	RUSTFLAGS=-Awarnings RUST_BACKTRACE=1 cargo test --features watch

release:
	 env OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_INCLUDE_DIR=/usr/local/include OPENSSL_STATIC=yes cargo build --release
