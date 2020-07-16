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
	cargo clippy --all-targets --features 'complete debug' -- -D warnings -A clippy::redundant_field_names -A clippy::useless_format -A clippy::let_and_return -A clippy::type_complexity -A clippy::option_map_unit_fn -A clippy::identity_conversion -A clippy::expect_fun_call -A clippy::option-as-ref-deref

src/db_conn_postgres.rs: src/db_conn_warning_header.txt src/db_conn_sqlite_new.header.rs src/db_conn_postgres.header.rs src/db_conn.base.rs
	cd src; ./generate_connectors.sh

doc: src/db_conn_postgres.rs
	cargo doc --no-deps	
	echo '<meta http-equiv="refresh" content="0; url=medal">' > target/doc/index.html
