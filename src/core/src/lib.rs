#![allow(private_bounds)]
mod adapters;
mod loyalty;
mod order_confirmed;
mod retrieve_loyalty_account;
mod spend_loyalty_points;
mod observability;

pub use adapters::ApplicationAdapters;
pub use order_confirmed::{OrderConfirmed, OrderConfirmedEventHandler};
pub use loyalty::{LoyaltyDto, LoyaltyAccountTransaction, LoyaltyErrors};
pub use retrieve_loyalty_account::RetrieveLoyaltyAccountQueryHandler;
pub use spend_loyalty_points::SpendLoyaltyPointsCommand;
pub use observability::{dd_observability, otlp_observability, use_datadog, log_observability, use_otlp, configure_instrumentation};