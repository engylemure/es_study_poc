mod lib;
use lib::*;
mod actix_app;
mod warp_app;
use std::net::Ipv4Addr;

pub fn server_address() -> (Ipv4Addr, u16) {
    (
        std::env::var("SERVER_HOST")
            .unwrap_or(String::from("127.0.0.1"))
            .parse()
            .expect("SERVER_HOST should be a valid Ip v4"),
        std::env::var("SERVER_PORT")
            .unwrap_or(String::from("8080"))
            .parse()
            .expect("SERVER_PORT should be a u16"),
    )
}

pub fn db_cfg() -> (String, u16) {
    (
        std::env::var("DB_HOST").unwrap_or(String::from("localhost")),
        std::env::var("DB_HOST")
            .unwrap_or(String::from("9200"))
            .parse()
            .expect("DB_PORT should be a number!"),
    )
}

// #[tokio::main]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var(
        "RUST_LOG",
        " tracing=info,warp=info,app=info,actix_web=info,actix_server=info",
    );
    env_logger::init();
    let (db_host, db_port) = db_cfg();
    let es_client = ElasticSearchClient::new(db_host, db_port);
    match std::env::var("ACTIX_SERVER")
        .unwrap_or(String::from("false"))
        .parse::<bool>()
        .unwrap()
    {
        true => actix_app::main(es_client, server_address()).await,
        false => warp_app::main(es_client, server_address()).await,
    }
}