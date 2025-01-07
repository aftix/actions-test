use std::{
    env::{self, VarError},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use tracing_subscriber::EnvFilter;
use warp::{filters::path::FullPath, reply::Response, Filter};

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const LOCALHOST_V6: IpAddr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));

#[tokio::main]
async fn main() {
    let span = tracing::span!(tracing::Level::INFO, "GHA proxy server");
    let _guard = span.enter();

    if let Ok(filter) = EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt::fmt()
            .compact()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt::fmt().compact().init();
    }

    let get_proxy = warp::any()
        .and(warp::get())
        .and(warp::path::full())
        .then(proxy_webhook)
        .with(warp::filters::compression::gzip());
    let post_proxy = warp::any()
        .and(warp::get())
        .and(warp::path::full())
        .then(proxy_webhook)
        .with(warp::filters::compression::gzip());
    let proxy = get_proxy.or(post_proxy);

    let listen_port = env::var("LISTEN_PORT").expect("Failed to get LISTEN_PORT envvar");
    let listen_port: u16 = listen_port
        .parse()
        .expect("LISTEN_PORT is not an unsigned integer");

    let cert_path = env::var("CERT_PATH");
    let key_path = env::var("KEY_PATH");

    match (cert_path, key_path) {
        (Ok(ref cert_path), Ok(ref key_path)) => {
            tracing::info!("Listening on port {listen_port}");
            tokio::join!(
                warp::serve(proxy.with(warp::trace::named("tls-ipv4")))
                    .tls()
                    .key_path(key_path)
                    .cert_path(cert_path)
                    .run((LOCALHOST, listen_port)),
                warp::serve(proxy.with(warp::trace::named("tls-ipv6")))
                    .tls()
                    .key_path(key_path)
                    .cert_path(cert_path)
                    .run((LOCALHOST_V6, listen_port)),
            );
        }
        (Ok(_), Err(VarError::NotPresent)) => panic!("If CERT_PATH is set, KEY_PATH must also be"),
        (Err(VarError::NotPresent), Ok(_)) => panic!("If KEY_PATH is set, CERT_PATH must also be"),
        (Err(VarError::NotPresent), Err(VarError::NotPresent)) => {
            #[cfg(debug_assertions)]
            tracing::warn!("Running in release mode without using TLS");

            tracing::info!("Listening on port {listen_port}");
            tokio::join!(
                warp::serve(proxy.with(warp::trace::named("ipv4"))).run((LOCALHOST, listen_port)),
                warp::serve(proxy.with(warp::trace::named("ipv6")))
                    .run((LOCALHOST_V6, listen_port)),
            );
        }
        _ => panic!("Inavlid unicode in KEY_PATH, CERT_PATH, or both"),
    }
}

async fn proxy_webhook(_path: FullPath) -> Response {
    todo!()
}
