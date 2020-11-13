#!/bin/sh

cat db_conn_warning_header.txt db_conn_sqlite_new.header.rs db_conn.base.rs | sed 's/\$/\?/g' | sed 's/{ "postgres" }/{ "sqlite_v2" }/' | sed 's/batch_execute/execute_batch/' | sed 's/ILIKE/LIKE/' > db_conn_sqlite_new.rs
cat db_conn_warning_header.txt db_conn_postgres.header.rs db_conn.base.rs > db_conn_postgres.rs
