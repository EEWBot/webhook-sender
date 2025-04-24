use std::net::{Ipv4Addr, SocketAddr};

use clap::Parser;

mod conn;
mod conn_initializer;
mod discord;
mod limiter;
mod request;
mod web;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, env, value_delimiter = ',', default_value = "0.0.0.0")]
    sender_ips: Vec<Ipv4Addr>,

    #[clap(long, env, value_delimiter = ',', default_value = "0.0.0.0")]
    retry_ips: Vec<Ipv4Addr>,

    #[clap(long, env, default_value_t = 1)]
    multiplier: u8,

    #[clap(long, env, default_value_t = 1)]
    rty_multiplier: u8,

    #[clap(long, env, default_value = "0.0.0.0:3000")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let format = tracing_subscriber::fmt::format()
        .with_target(false)
        .compact();

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(tracing::Level::INFO)
        .init();

    let (sender, limiter) = conn_initializer::initialize(
        &cli.retry_ips,
        &cli.sender_ips,
        cli.multiplier,
        cli.rty_multiplier,
    )
    .await
    .expect("failed to initialize connection");

    web::run(cli.listen, sender, limiter).await.unwrap();
}
