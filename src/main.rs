use std::net::Ipv4Addr;
use std::sync::Arc;

use clap::Parser;

mod conn;
mod conn_initializer;
mod discord;
mod limiter;
mod request;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, env, value_delimiter = ',', default_value = "0.0.0.0")]
    sender_ips: Vec<Ipv4Addr>,

    #[clap(long, env, value_delimiter = ',', default_value = "0.0.0.0")]
    retry_ips: Vec<Ipv4Addr>,

    #[clap(long, env, default_value_t = 4)]
    multiplier: u8,
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

    let (sender, _limiter) =
        conn_initializer::initialize(&cli.retry_ips, &cli.sender_ips, cli.multiplier)
            .await
            .expect("failed to initialize connection");

    let targets: Vec<http::Uri> = include_str!("../targets.txt")
        .split('\n')
        .filter_map(|v| v.parse().ok())
        .collect();

    std::thread::sleep(std::time::Duration::from_secs(3));

    loop {
        tracing::info!("Start");

        let body = bytes::Bytes::from(format!(
            "{{\"content\": \"<@349168429980188672> もげもげきゅんっ！\"}}"
        ));

        let context = Arc::new(request::Context {
            retry_limit: 3,
            body,
        });

        for _ in 0..4 {
            for target in &targets {
                // let target = targets.choose(&mut rng).unwrap();

                let request = request::Request {
                    context: context.clone(),
                    retry_count: 0,
                    target: target.to_string(),
                };

                sender.send(request).await.unwrap();
            }
        }

        drop(context);

        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
