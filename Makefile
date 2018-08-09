all:
	RUSTFLAGS=-Awarnings RUST_BACKTRACE=1 cargo run --features watch
