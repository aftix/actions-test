use bytes::{Buf, Bytes};
use serde::Serialize;
use std::{
    env::{self, VarError},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use warp::{
    filters::path::FullPath,
    http::{HeaderMap, Request, StatusCode},
    hyper::{Body, Client},
    reply::{with_status, Reply, Response},
    Filter,
};

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

    let args_proxy = warp::path::full()
        .and(warp::filters::header::optional("X-Github-Event"))
        .and(warp::filters::header::headers_cloned());

    let proxy = warp::any()
        .and(warp::post())
        .and(args_proxy)
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::bytes())
        .then(proxy_webhook)
        .with(warp::filters::compression::gzip());

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

#[derive(Serialize)]
struct RepositoryDispatch {
    event_type: String,
    client_payload: serde_json::Value,
}

async fn proxy_webhook(
    path: FullPath,
    event_header: Option<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    debug!("Got POST request for {:?}", path);

    let event_type = event_header.unwrap_or("workflow_dispatch".to_owned());

    // SAFETY: 400 is a valid status code
    let http400 = unsafe { StatusCode::from_u16(400).unwrap_unchecked() };
    // SAFETY: 500 is a valid status code
    let http500 = unsafe { StatusCode::from_u16(500).unwrap_unchecked() };

    let body = serde_json::from_reader::<_, serde_json::Value>(body.reader());

    let Ok(request_body) = body else {
        return with_status("Inavalid body", http400).into_response();
    };
    debug!("Got body from POST request");

    let mut builder = Request::post(format!("https://github.com/{}", path.as_str()))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    for (header, value) in &headers {
        builder = builder.header(header, value);
    }

    let request_body = RepositoryDispatch {
        event_type,
        client_payload: request_body,
    };
    let Ok(request_body) = serde_json::ser::to_vec(&request_body) else {
        return with_status("Unable to re-serialize body", http500).into_response();
    };
    let request_body = Body::from(request_body);
    debug!("Serialized response body");

    let request = match builder.body(request_body) {
        Err(err) => {
            return with_status(err.to_string(), http500).into_response();
        }
        Ok(req) => req,
    };
    info!("Created request for github");

    let client = Client::new();

    let Ok(response) = client.request(request).await else {
        return with_status("Failed to send request to github", http500).into_response();
    };
    info!("Got response from github");

    response
}
