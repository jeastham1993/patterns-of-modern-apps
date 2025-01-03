use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use lambda_http::run;
use loyalty_adapters::{configure_instrumentation, ApplicationAdapters, PostgresLoyaltyPoints};
use loyalty_core::{
    LoyaltyDto, LoyaltyErrors, LoyaltyPoints,RetrieveLoyaltyAccountQueryHandler, SpendLoyaltyPointsCommand,
    SpendLoyaltyPointsCommandHandler,
};
use tracing::info;

pub enum HostingOption {
    Lambda,
    HttpServer,
}

impl HostingOption {
    async fn run(&self, app: Router) {
        match self {
            HostingOption::Lambda => {
                info!("Starting app using Lambda runtime and `lambda_http` crate.");

                let _ = run(app).await;
            }
            HostingOption::HttpServer => {
                let port = std::env::var("PORT").unwrap_or("8080".to_string());

                tracing::info!("Starting web server on port {}", port);

                let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                    .await
                    .unwrap();

                axum::serve(listener, app.into_make_service())
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .unwrap();
            }
        }
    }
}

pub struct AppState<T: LoyaltyPoints + Send + Sync> {
    pub application: ApplicationAdapters<T>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = configure_instrumentation();

    let database = PostgresLoyaltyPoints::new().await?;

    let application_adapters = ApplicationAdapters::new(database).await;

    let shared_state = Arc::new(AppState {
        application: application_adapters,
    });

    let app = Router::new()
        .route("/loyalty/:customer_id", get(get_loyalty_points))
        .route("/loyalty/:customer_id/spend", post(spend_loyalty_points))
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .with_state(shared_state);

    let hosting = get_hosting_option();
    hosting.run(app).await;

    Ok(())
}

#[tracing::instrument(name = "get_loyalty_points", skip(state, path))]
async fn get_loyalty_points<T: LoyaltyPoints + Send + Sync>(
    State(state): State<Arc<AppState<T>>>,
    path: Path<String>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points =
        RetrieveLoyaltyAccountQueryHandler::handle(&state.application.loyalty_points, path.0).await;

    match loyalty_points {
        Ok(loyalty) => (StatusCode::OK, (Json(Some(loyalty)))),
        Err(_) => (StatusCode::NOT_FOUND, Json(None)),
    }
}

#[tracing::instrument(name = "spend_loyalty_points", skip(state, payload), fields(span.kind="server"))]
async fn spend_loyalty_points<T: LoyaltyPoints + Send + Sync>(
    State(state): State<Arc<AppState<T>>>,
    Json(payload): Json<SpendLoyaltyPointsCommand>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points =
        SpendLoyaltyPointsCommandHandler::handle(&state.application.loyalty_points, payload).await;

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

fn get_hosting_option() -> HostingOption {
    match std::env::var("LAMBDA_TASK_ROOT") {
        Ok(_) => HostingOption::Lambda,
        Err(_) => HostingOption::HttpServer,
    }
}
