use std::sync::Arc;

use serde::Deserialize;

use crate::{
    loyalty::{LoyaltyErrors, LoyaltyPoints},
    LoyaltyDto,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendLoyaltyPointsCommand {
    customer_id: String,
    order_number: String,
    spend: f32,
}

pub struct SpendLoyaltyPointsCommandHandler<T: LoyaltyPoints + 'static> {
    loyalty_points: Arc<T>,
}

impl<T: LoyaltyPoints> SpendLoyaltyPointsCommandHandler<T> {
    pub async fn new(loyalty_points: Arc<T>) -> Self {
        Self { loyalty_points }
    }

    #[tracing::instrument(name = "handle_spend_loyalty_points", skip(self, command), fields(customer_id=command.customer_id, order_number=command.order_number, spend=command.spend))]
    pub async fn handle(
        &self,
        command: SpendLoyaltyPointsCommand,
    ) -> anyhow::Result<LoyaltyDto, LoyaltyErrors> {
        let mut account = self.loyalty_points.retrieve(&command.customer_id).await?;

        let transaction = account.spend_points(&command.order_number, &command.spend)?;

        let _ = &self
            .loyalty_points
            .add_transaction(&account, transaction)
            .await?;

        Ok(account.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::loyalty::{LoyaltyAccount, MockLoyaltyPoints};

    use super::*;
    use mockall::predicate;

    #[tokio::test]
    async fn on_valid_command_points_should_be_added() {
        let test_customer_id = "james";
        let customer_existing_points = 10.0;
        let customer_spend = 5.0;

        let mut loyalty_points = MockLoyaltyPoints::new();
        loyalty_points
            .expect_retrieve()
            .with(predicate::eq(test_customer_id))
            .times(1)
            .returning(move |customer_id| {
                LoyaltyAccount::from(customer_id.to_string(), customer_existing_points, vec![])
            });
        loyalty_points
            .expect_add_transaction()
            .times(1)
            .returning(|_, _| Ok(()));

        let command = SpendLoyaltyPointsCommand {
            customer_id: test_customer_id.to_string(),
            order_number: "ORD123".to_string(),
            spend: customer_spend,
        };
        let handler = SpendLoyaltyPointsCommandHandler::new(Arc::new(loyalty_points)).await;

        let result = handler.handle(command).await;

        let account = result.unwrap();

        assert_eq!(account.current_points, 5.0);
        assert_eq!(account.transactions.len(), 1);
    }

    #[tokio::test]
    async fn on_valid_command_points_when_points_arent_available_should_error() {
        let test_customer_id = "james";
        let customer_existing_points = 5.0;
        let customer_spend = 10.0;

        let mut loyalty_points = MockLoyaltyPoints::new();
        loyalty_points
            .expect_retrieve()
            .with(predicate::eq(test_customer_id))
            .times(1)
            .returning(move |customer_id| {
                LoyaltyAccount::from(customer_id.to_string(), customer_existing_points, vec![])
            });
        loyalty_points.expect_add_transaction().times(0);

        let command = SpendLoyaltyPointsCommand {
            customer_id: test_customer_id.to_string(),
            order_number: "ORD123".to_string(),
            spend: customer_spend,
        };
        let handler = SpendLoyaltyPointsCommandHandler::new(Arc::new(loyalty_points)).await;

        let result = handler.handle(command).await;

        assert_eq!(result.is_err(), true);
    }

    #[tokio::test]
    async fn on_valid_command_points_when_account_not_found_should_error() {
        let test_customer_id = "james";
        let customer_spend = 10.0;

        let mut loyalty_points = MockLoyaltyPoints::new();
        loyalty_points
            .expect_retrieve()
            .with(predicate::eq(test_customer_id))
            .times(1)
            .returning(move |_| Err(LoyaltyErrors::AccountNotFound()));
        loyalty_points.expect_add_transaction().times(0);

        let command = SpendLoyaltyPointsCommand {
            customer_id: test_customer_id.to_string(),
            order_number: "ORD123".to_string(),
            spend: customer_spend,
        };
        let handler = SpendLoyaltyPointsCommandHandler::new(Arc::new(loyalty_points)).await;

        let result = handler.handle(command).await;

        assert_eq!(result.is_err(), true);
    }
}
