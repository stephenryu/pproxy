use anyhow::{Context, Result};
use log::{error, info};
use std::{io::ErrorKind, net::SocketAddr};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};

use crate::config::{Config, ProxyRule};

pub(crate) async fn run(conf: Config) -> Result<()> {
    let pproxys = conf.proxy_list.len();
    info!("PProxy Starting... {pproxys} proxy(s) configured");

    if pproxys == 0 {
        info!("No proxies configured. Exiting.");
        return Ok(());
    }

    let mut started = 0usize;
    for rule in conf.proxy_list.clone() {
        match bind_listener(&rule).await? {
            Some(listener) => {
                started += 1;
                tokio::spawn(async move {
                    if let Err(err) = start_proxy_server(rule, listener).await {
                        error!("proxy server exited with error: {err:#}");
                    }
                });
            }
            None => {
                // bind_listener already logged why it was skipped.
            }
        }
    }

    if started == 0 {
        error!("No proxy listeners could be started. Exiting.");
        return Ok(());
    }

    wait_for_shutdown_signal().await;
    info!("Shutting down...");
    Ok(())
}

async fn bind_listener(rule: &ProxyRule) -> Result<Option<TcpListener>> {
    let listen_addr = format!("{}:{}", rule.listen_host, rule.listen_port);

    let listener = match TcpListener::bind(&listen_addr).await {
        Ok(listener) => listener,
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            error!(
                "{} is already in use; cannot bind it. Stop the process using it or change listen_host/listen_port.",
                listen_addr
            );
            return Ok(None);
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to bind to {listen_addr}"));
        }
    };

    info!("Proxy server started on {}", listen_addr);
    Ok(Some(listener))
}

async fn start_proxy_server(rule: ProxyRule, listener: TcpListener) -> Result<()> {
    info!(
        "Starting proxy server on {}:{} with target server port: {}",
        rule.listen_host, rule.listen_port, rule.target_port
    );

    loop {
        let (client, remote_addr) = listener.accept().await?;
        info!("New client connected {remote_addr}");

        let target = format!("{}:{}", rule.target_host, rule.target_port);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(client, &target, remote_addr).await {
                error!("connection error: {err:#}");
            }
        });
    }
}

async fn handle_connection(
    mut client: TcpStream,
    target_addr: &str,
    remote_addr: SocketAddr,
) -> Result<()> {
    info!("Connecting to target server: {target_addr}");

    let mut target = TcpStream::connect(target_addr)
        .await
        .with_context(|| format!("failed to connect to target {target_addr}"))?;

    let _ = copy_bidirectional(&mut client, &mut target).await;

    info!("Client disconnected {remote_addr}");
    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint = signal(SignalKind::interrupt()).expect("sigint handler");
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm handler");
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
