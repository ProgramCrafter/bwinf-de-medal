all:
	RUSTFLAGS=-Awarnings RUST_BACKTRACE=1 cargo run --features watch

test:
	RUSTFLAGS=-Awarnings RUST_BACKTRACE=1 cargo test
