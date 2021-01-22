mod lib;
mod actix_app;
mod warp_app;

fn main() -> std::io::Result<()> {
    std::env::set_var(
        "RUST_LOG",
        " tracing=info,warp=info,app=info,actix_web=info,actix_server=info",
    );
    env_logger::init();
    match std::env::var("ACTIX_SERVER")
        .unwrap_or(String::from("false"))
        .parse::<bool>()
        .unwrap()
    {
        true => actix_app::main(),
        false => warp_app::main(),
    }
}