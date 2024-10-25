use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use loyalty_core::{
    configure_instrumentation, ApplicationAdapters, LoyaltyDto, LoyaltyErrors,
    SpendLoyaltyPointsCommand,
};

pub struct AppState {
    pub application: Arc<ApplicationAdapters>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    let (_provider, _subscriber) = configure_instrumentation();

    let application_adapters = ApplicationAdapters::new().await;

    let shared_state = Arc::new(AppState {
        application: Arc::new(application_adapters),
    });

    let app = Router::new()
        .route("/loyalty/:customer_id", get(get_loyalty_points))
        .route("/loyalty/:customer_id/spend", post(spend_loyalty_points))
        .layer(OtelInResponseLayer::default())
        .layer(OtelAxumLayer::default())
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

#[tracing::instrument(name = "get_loyalty_points", skip(state, path))]
async fn get_loyalty_points(
    State(state): State<Arc<AppState>>,
    path: Path<String>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points = state
        .application
        .retrieve_loyalty_query_handler
        .handle(path.0)
        .await;

    match loyalty_points {
        Ok(loyalty) => (StatusCode::OK, (Json(Some(loyalty)))),
        Err(_) => (StatusCode::NOT_FOUND, Json(None)),
    }
}

#[tracing::instrument(name = "spend_loyalty_points", skip(state, payload))]
async fn spend_loyalty_points(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<SpendLoyaltyPointsCommand>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points = state.application.spend_points_handler.handle(payload).await;

    match loyalty_points {
        Ok(account) => (StatusCode::OK, (Json(Some(account)))),
        Err(e) => match e {
            LoyaltyErrors::PointsNotAvailable(_) => (StatusCode::BAD_REQUEST, (Json(None))),
            LoyaltyErrors::AccountNotFound() => (StatusCode::NOT_FOUND, (Json(None))),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, (Json(None))),
        },
    }
}

async fn shutdown_signal() {
    use std::sync::mpsc;
    use std::{thread, time::Duration};

    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
    let (sender, receiver) = mpsc::channel();
    let _ = thread::spawn(move || {
        opentelemetry::global::shutdown_tracer_provider();
        sender.send(()).ok()
    });
    let shutdown_res = receiver.recv_timeout(Duration::from_millis(2_000));
    if shutdown_res.is_err() {
        tracing::error!("failed to shutdown OpenTelemetry");
    }
}
