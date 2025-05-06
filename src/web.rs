use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context as _, Result as AHResult};
use axum::{
    Router,
    extract::{Json, State},
    http::{
        StatusCode,
        header::{CONTENT_TYPE, HeaderValue},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::TypedHeader;
use bytes::Bytes;
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::limiter::Limiter;
use crate::request::{Context, JobSender, Request};

#[derive(Clone, Debug)]
struct AppState {
    sender: JobSender,
    limiter: &'static Limiter,
    auth_token: String,
}

#[derive(Clone, Debug, Deserialize)]
struct WebRequest {
    targets: Vec<url::Url>,
    body: serde_json::Value,
    retry_limit: Option<usize>,
}

async fn get_notfounds(
    State(app): State<AppState>,
    TypedHeader(token): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
) -> Response {
    if token.0.token() != app.auth_token {
        return (StatusCode::UNAUTHORIZED, "UNAUTHORIZED").into_response();
    }

    Json(app.limiter.notfounds()).into_response()
}

async fn delete_notfounds(
    State(app): State<AppState>,
    TypedHeader(token): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    Json(targets): Json<Vec<url::Url>>,
) -> Response {
    if token.0.token() != app.auth_token {
        return (StatusCode::UNAUTHORIZED, "UNAUTHORIZED").into_response();
    }

    tokio::spawn(async move {
        tracing::info!("Clear {} 404 targets scheduled after 60(s)!", targets.len());
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        app.limiter.clear_notfounds(&targets);
    });

    "OK".into_response()
}

#[axum::debug_handler]
async fn send(
    State(app): State<AppState>,
    TypedHeader(token): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    Json(requests): Json<Vec<WebRequest>>,
) -> Response {
    if token.0.token() != app.auth_token {
        return (StatusCode::UNAUTHORIZED, "UNAUTHORIZED").into_response();
    }

    let my_requests = {
        let mut rng = rand::rng();

        let queuing_id = crate::namesgenerator::generate(&mut rng);

        let mut my_requests = vec![];

        for request in requests {
            let request_id = crate::namesgenerator::generate(&mut rng);
            tracing::info!(
                "{queuing_id}#{request_id} Queuing {} targets",
                request.targets.len()
            );

            let body = Bytes::from(request.body.to_string().into_bytes());
            let context = Arc::new(Context {
                identity: format!("{queuing_id}#{request_id}"),
                body,
                retry_limit: request.retry_limit.unwrap_or(10),
            });

            for target in request.targets {
                let target_id = crate::namesgenerator::generate(&mut rng);

                my_requests.push(Request {
                    context: context.clone(),
                    retry_count: 0,
                    target,
                    identity: format!("{queuing_id}#{request_id}#{target_id}"),
                });
            }
        }

        my_requests
    };

    for request in my_requests {
        app.sender
            .send(request)
            .await
            .expect("Failed to send Request");
    }

    "OK".into_response()
}

async fn root() -> Response {
    (
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        "<h1>Welcome to Webhook Sender</h1>",
    )
        .into_response()
}

pub async fn run(
    listen: SocketAddr,
    sender: JobSender,
    limiter: &'static Limiter,
    auth_token: &str,
) -> AHResult<()> {
    let auth_token = auth_token.to_owned();

    let app = Router::new()
        .route("/", get(root))
        .route("/api/send", post(send))
        .route(
            "/api/notfounds",
            get(get_notfounds).delete(delete_notfounds),
        )
        .with_state(AppState {
            sender,
            limiter,
            auth_token,
        });

    let listener = TcpListener::bind(listen)
        .await
        .context("Failed to bind address")?;

    axum::serve(listener, app)
        .await
        .context("Failed to serve HTTP contents")?;

    Ok(())
}
