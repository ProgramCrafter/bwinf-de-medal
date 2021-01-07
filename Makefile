debug: src/db_conn_postgres.rs
	RUST_BACKTRACE=1 cargo run --features 'watch debug' -- -a

pgdebug: src/db_conn_postgres.rs
	RUST_BACKTRACE=1 cargo run --features 'watch debug postgres' -- -a -D 'postgres://medal:medal@localhost/medal'

test: src/db_conn_postgres.rs
	RUST_BACKTRACE=1 cargo test --features 'complete debug'

release: src/db_conn_postgres.rs
	env OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_INCLUDE_DIR=/usr/local/include OPENSSL_STATIC=yes cargo build --release --features 'server'

stricttest: src/db_conn_postgres.rs
	cargo test --features 'strict complete debug' --verbose

dynrelease: src/db_conn_postgres.rs
	cargo build --release --features 'server'

format: src/db_conn_postgres.rs
	cargo +nightly fmt

clippy: src/db_conn_postgres.rs
	cargo clippy --all-targets --features 'complete debug' -- -D warnings -A clippy::type-complexity -A clippy::option-map-unit-fn -A clippy::len-zero -A clippy::option-as-ref-deref -A clippy::or-fun-call -A clippy::comparison-to-empty -A clippy::result-unit-err

src/db_conn_postgres.rs: src/db_conn_warning_header.txt src/db_conn_sqlite_new.header.rs src/db_conn_postgres.header.rs src/db_conn.base.rs
	cd src; ./generate_connectors.sh

doc: src/db_conn_postgres.rs
	cargo doc --no-deps	
	echo '<meta http-equiv="refresh" content="0; url=medal">' > target/doc/index.html

grcov: src/db_conn_postgres.rs
	CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort" cargo +nightly test
	grcov ./target/debug/ -s . -t html --llvm --branch --ignore-not-existing -o ./target/debug/coverage/
