use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context as _, Result as AHResult};
use axum::{
    Router,
    extract::{Json, State},
    http::header::{CONTENT_TYPE, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bytes::Bytes;
use names::Generator;
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::limiter::Limiter;
use crate::request::{Context, JobSender, Request};

#[derive(Clone, Debug)]
struct AppState {
    sender: JobSender,
    limiter: &'static Limiter,
}

#[derive(Clone, Debug, Deserialize)]
struct WebRequest {
    targets: Vec<String>,
    body: serde_json::Value,
    retry_limit: Option<usize>,
}

async fn notfounds(State(app): State<AppState>) -> Json<Vec<String>> {
    Json(app.limiter.notfounds())
}

#[axum::debug_handler]
async fn send(State(app): State<AppState>, Json(requests): Json<Vec<WebRequest>>) -> Response {
    let my_requests = {
        let mut generator = Generator::default();

        let queuing_id = generator.next().unwrap();

        let mut my_requests = vec![];

        for request in requests {
            let request_id = generator.next().unwrap();
            tracing::info!("Queuing {queuing_id}#{request_id}");

            let body = Bytes::from(request.body.to_string().into_bytes());
            let context = Arc::new(Context {
                identity: format!("{queuing_id}#{request_id}"),
                body,
                retry_limit: request.retry_limit.unwrap_or(10),
            });

            for target in request.targets {
                let target_id = generator.next().unwrap();

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

pub async fn run(listen: SocketAddr, sender: JobSender, limiter: &'static Limiter) -> AHResult<()> {
    let app = Router::new()
        .route("/", get(root))
        .route("/api/send", post(send))
        .route("/api/notfounds", get(notfounds))
        .with_state(AppState { sender, limiter });

    let listener = TcpListener::bind(listen)
        .await
        .context("Failed to bind address")?;

    axum::serve(listener, app)
        .await
        .context("Failed to serve HTTP contents")?;

    Ok(())
}
