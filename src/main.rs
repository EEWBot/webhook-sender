use std::net::{SocketAddr, Ipv4Addr};

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

    #[clap(long, env, default_value = "0.0.0.0:3000")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(true)
        .compact();

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(tracing::Level::INFO)
        .init();

    let (sender, limiter) =
        conn_initializer::initialize(&cli.retry_ips, &cli.sender_ips, cli.multiplier)
            .await
            .expect("failed to initialize connection");

    web::run(cli.listen, sender, limiter).await.unwrap();

    // let mut targets: Vec<http::Uri> = include_str!("../targets.txt")
    //     .split('\n')
    //     .filter_map(|v| v.parse().ok())
    //     .collect();
    // targets.push("https://ptb.discord.com/api/webhooks/1231506323150209096/Q7zFmcmy8rQ8KTrGv2esqAtUxom13ir4GmBNN0TqpQjeEWXwF51xQzmDUzC6UGhSlqlt".parse().unwrap());
    //
    // std::thread::sleep(std::time::Duration::from_secs(3));
    //
    // let mut counter = 0;
    // loop {
    //     counter += 1;
    //
    //     tracing::info!("Start {counter}");
    //
    //     let body = bytes::Bytes::from(format!(
    //         "{{\"content\": \"<@349168429980188672> もげもげきゅんっ！ {counter}\"}}"
    //     ));
    //
    //     let context = Arc::new(request::Context {
    //         retry_limit: 3,
    //         body,
    //     });
    //
    //     for target in &targets {
    //         let request = request::Request {
    //             context: context.clone(),
    //             retry_count: 0,
    //             target: target.to_string(),
    //         };
    //
    //         sender.send(request).await.unwrap();
    //     }
    //
    //     drop(context);
    //
    //     std::thread::sleep(std::time::Duration::from_secs(2));
    // }
}
