use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

use anyhow::{Context, Result as AHResult};
use hickory_resolver::{Resolver, config::ResolverConfig, name_server::TokioConnectionProvider};

use crate::limiter::Limiter;
use crate::request::JobSender;

async fn query_discord_ips() -> AHResult<Vec<Ipv4Addr>> {
    let resolver = Resolver::builder_with_config(
        ResolverConfig::default(),
        TokioConnectionProvider::default(),
    )
    .build();

    let mut ips = vec![];
    let response = resolver
        .lookup_ip("discord.com")
        .await
        .context("Failed to resolve discord.com")?;

    ips.extend(response.iter().map(|ip| match ip {
        IpAddr::V4(ip) => ip,
        _ => panic!("WTF!? discord.com provides IPv6 Addr"),
    }));

    tracing::info!("I got {} ips in discord.com! {ips:?}", ips.len());

    Ok(ips)
}

pub async fn initialize(
    retry_ips: &[Ipv4Addr],
    sender_ips: &[Ipv4Addr],
    multiplier: u8,
) -> AHResult<(JobSender, &'static Limiter)> {
    let target_ips = query_discord_ips().await?;

    let target_socks: Vec<_> = target_ips
        .iter()
        .map(|ip| SocketAddrV4::new(*ip, 443))
        .collect();

    let retry_socks: Vec<_> = retry_ips
        .iter()
        .map(|ip| SocketAddrV4::new(*ip, 0))
        .collect();

    let sender_socks: Vec<_> = sender_ips
        .iter()
        .map(|ip| SocketAddrV4::new(*ip, 0))
        .collect();

    let limiter = &*Box::leak(Box::new(Limiter::default()));

    let (retry_tx, retry_rx) = async_channel::unbounded();
    let (tx, rx) = async_channel::unbounded();

    for _ in 0..multiplier {
        for from in &sender_socks {
            for to in &target_socks {
                let rx = rx.clone();
                let tx = retry_tx.clone();
                let from = *from;
                let to = *to;

                tokio::spawn(async move {
                    crate::conn::sender_loop(from, to, rx, tx, limiter).await;
                });
            }
        }

        for from in &retry_socks {
            for to in &target_socks {
                let rx = retry_rx.clone();
                let tx = retry_tx.clone();
                let from = *from;
                let to = *to;

                tokio::spawn(async move {
                    crate::conn::sender_loop(from, to, rx, tx, limiter).await;
                });
            }
        }
    }

    Ok((tx, limiter))
}
