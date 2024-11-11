use std::sync::Arc;

use adapters::D1DataAccessLayer;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use loyalty_core::{
    LoyaltyDto, LoyaltyErrors, LoyaltyPoints, OrderConfirmed, OrderConfirmedEventHandler,
    RetrieveLoyaltyAccountQueryHandler, SpendLoyaltyPointsCommand,
    SpendLoyaltyPointsCommandHandler,
};
use tower_service::Service;
use tracing_subscriber::{fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_web::{performance_layer, MakeConsoleWriter};
use worker::*;

mod adapters;

pub struct AppState<T: LoyaltyPoints + Send + Sync> {
    pub loyalty_points: T,
}

#[event(start)]
fn start() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false) // Only partially supported across JavaScript runtimes
        .without_time()
        .with_writer(MakeConsoleWriter); // write events to the console

    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();

    let db = env.d1("DB")?;

    let postgres_db = D1DataAccessLayer::new(db).await;

    let shared_state = Arc::new(AppState {
        loyalty_points: postgres_db,
    });

    let mut app: Router = Router::new()
        .route("/loyalty/:customer_id", get(get_loyalty_points))
        .route("/loyalty/:customer_id/spend", post(spend_loyalty_points))
        .with_state(shared_state);

    Ok(app.call(req).await?)
}

async fn get_loyalty_points<T: LoyaltyPoints + Send + Sync>(
    State(state): State<Arc<AppState<T>>>,
    path: Path<String>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points =
        RetrieveLoyaltyAccountQueryHandler::handle(&state.loyalty_points, path.0).await;

    match loyalty_points {
        Ok(loyalty) => (StatusCode::OK, (Json(Some(loyalty)))),
        Err(_) => (StatusCode::NOT_FOUND, Json(None)),
    }
}

async fn spend_loyalty_points<T: LoyaltyPoints + Send + Sync>(
    State(state): State<Arc<AppState<T>>>,
    Json(payload): Json<SpendLoyaltyPointsCommand>,
) -> (StatusCode, Json<Option<LoyaltyDto>>) {
    let loyalty_points =
        SpendLoyaltyPointsCommandHandler::handle(&state.loyalty_points, payload).await;

    match loyalty_points {
        Ok(account) => (StatusCode::OK, (Json(Some(account)))),
        Err(e) => match e {
            LoyaltyErrors::PointsNotAvailable(_) => (StatusCode::BAD_REQUEST, (Json(None))),
            LoyaltyErrors::AccountNotFound() => (StatusCode::NOT_FOUND, (Json(None))),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, (Json(None))),
        },
    }
}

#[event(queue)]
pub async fn main(message_batch: MessageBatch<OrderConfirmed>, env: Env, _: Context) -> Result<()> {
    console_error_panic_hook::set_once();

    let db = env.d1("DB")?;

    let postgres_db = D1DataAccessLayer::new(db).await;

    for message in message_batch.messages()? {
        let res = OrderConfirmedEventHandler::handle(&postgres_db, message.body()).await;

        if res.is_ok() {
            message.ack();
        }
    }

    Ok(())
}
