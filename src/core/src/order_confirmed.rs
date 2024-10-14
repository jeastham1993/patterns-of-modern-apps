use serde::Deserialize;
use tracing::info;

use crate::loyalty::LoyaltyPoints;

#[derive(Deserialize)]
pub struct OrderConfirmed {
    customer_id: String,
    order_id: String,
    order_value: f32,
}

pub struct OrderConfirmedEventHandler<T: LoyaltyPoints> {
    loyalty_points: T,
}

impl<T: LoyaltyPoints> OrderConfirmedEventHandler<T> {
    pub async fn new(loyalty_points: T) -> Self {
        Self { loyalty_points }
    }

    pub async fn handle(&self, evt: OrderConfirmed) -> Result<(), ()> {
        info!(
            "Processing message for customer {} with id {} and value {}",
            evt.customer_id, evt.order_id, evt.order_value
        );

        let existing_account = self.loyalty_points.retrieve(&evt.customer_id).await;

        let mut account = match existing_account {
            Some(existing_account) => {
                info!("Existing loyalty account found");

                existing_account
            }
            None => {
                info!("Created new loyalty account");

                self.loyalty_points.new_account(evt.customer_id).await
            }
        };

        let transaction = account.add_transaction(evt.order_id, evt.order_value);

        let update_res = self
            .loyalty_points
            .add_transaction(account, transaction)
            .await;

        match update_res {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}
