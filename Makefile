debug:
	RUST_BACKTRACE=1 cargo run --features 'watch debug' -- -a

test:
	RUST_BACKTRACE=1 cargo test --features 'complete debug'

release:
	env OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_INCLUDE_DIR=/usr/local/include OPENSSL_STATIC=yes cargo build --release --features 'server'

stricttest:
	cargo test --features 'strict complete debug' --verbose

dynrelease:
	cargo build --release --features 'server'

format:
	cargo +nightly fmt

clippy:
	cargo clippy --all-targets --all-features -- -D warnings -A clippy::redundant_field_names -A clippy::useless_format -A clippy::let_and_return -A clippy::type_complexity -A clippy::option_map_unit_fn -A clippy::identity_conversion -A clippy::expect_fun_call


