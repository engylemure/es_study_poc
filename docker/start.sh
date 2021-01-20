dockerize -wait tcp://$DB_HOST:$DB_PORT
cargo run --release