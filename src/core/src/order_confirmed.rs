use serde::Deserialize;
use tracing::info;

use crate::loyalty::LoyaltyPoints;

#[derive(Deserialize)]
pub struct OrderConfirmed {
    customer_id: String,
    order_id: String,
    order_value: f32,
}

pub struct OrderConfirmedEventHandler {}

impl OrderConfirmedEventHandler {
    #[tracing::instrument(name = "handle_order_confirmed",skip(loyalty_points, evt), fields(customer_id=evt.customer_id, order_id=evt.order_id, order_value=evt.order_value))]
    pub async fn handle<T: LoyaltyPoints>(loyalty_points: &T, evt: &OrderConfirmed) -> Result<(), ()> {
        info!(
            "Processing message for customer {} with id {} and value {}",
            evt.customer_id, evt.order_id, evt.order_value
        );

        let existing_account = loyalty_points.retrieve(&evt.customer_id).await;

        let mut account = match existing_account {
            Ok(account) => {
                info!("Existing loyalty account found");

                account
            }
            Err(e) => match e {
                crate::loyalty::LoyaltyErrors::AccountNotFound() => {
                        loyalty_points
                        .new_account(evt.customer_id.clone())
                        .await
                        .map_err(|e| {
                            tracing::error!("Failure creating new account: {:?}", e);

                            
                        })?
                }
                crate::loyalty::LoyaltyErrors::InvalidValues(e)
                | crate::loyalty::LoyaltyErrors::PointsNotAvailable(e)
                | crate::loyalty::LoyaltyErrors::TransactionExistsForOrder(e)
                | crate::loyalty::LoyaltyErrors::DatabaseError(e) => {
                    tracing::error!("Failure retrieving account from database: {:?}", e);

                    return Err(());
                }
            },
        };

        let transaction = account.add_transaction(evt.order_id.clone(), evt.order_value);

        if transaction.is_ok() {
            let update_res = loyalty_points
                .add_transaction(&account, transaction.unwrap())
                .await;

            return match update_res {
                Ok(_) => Ok(()),
                Err(_) => Err(()),
            };
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        loyalty::{LoyaltyAccount, MockLoyaltyPoints},
        LoyaltyErrors,
    };

    use super::*;
    use mockall::predicate;

    #[tokio::test]
    async fn on_valid_event_for_new_customer_should_create_account_and_add_points() {
        let test_customer_id = "james";
        let test_order_id = "ORD987";
        let test_order_value = 100.00;

        let mut loyalty_points = MockLoyaltyPoints::new();
        loyalty_points
            .expect_retrieve()
            .with(predicate::eq(test_customer_id))
            .times(1)
            .returning(move |_| Err(LoyaltyErrors::AccountNotFound()));
        loyalty_points
            .expect_add_transaction()
            .times(1)
            .returning(|_, _| Ok(()));
        loyalty_points
            .expect_new_account()
            .times(1)
            .returning(LoyaltyAccount::new);

        let evt = OrderConfirmed {
            customer_id: test_customer_id.to_string(),
            order_id: test_order_id.to_string(),
            order_value: test_order_value,
        };

        let result = OrderConfirmedEventHandler::handle(&loyalty_points, &evt).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn on_valid_event_for_existing_customer_should_create_account_and_add_points() {
        let test_customer_id = "james";
        let test_order_id = "ORD987";
        let test_order_value = 100.00;

        let mut loyalty_points = MockLoyaltyPoints::new();
        loyalty_points
            .expect_retrieve()
            .with(predicate::eq(test_customer_id))
            .times(1)
            .returning(|customer_id| LoyaltyAccount::from(customer_id.to_string(), 10.0, vec![]));
        loyalty_points
            .expect_add_transaction()
            .times(1)
            .returning(|_, _| Ok(()));

        let evt = OrderConfirmed {
            customer_id: test_customer_id.to_string(),
            order_id: test_order_id.to_string(),
            order_value: test_order_value,
        };

        let result = OrderConfirmedEventHandler::handle(&loyalty_points, &evt).await;

        assert!(result.is_ok());
    }
}
