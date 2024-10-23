use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use loyalty_core::{ApplicationAdpaters, LoyaltyDto};

pub struct AppState {
    pub application: Arc<ApplicationAdpaters>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt().init();

    let application_adapters = ApplicationAdpaters::new().await;

    let shared_state = Arc::new(AppState {
        application: Arc::new(application_adapters),
    });

    let app = Router::new()
        .route("/loyalty/:customer_id", get(get_loyalty_points))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

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
