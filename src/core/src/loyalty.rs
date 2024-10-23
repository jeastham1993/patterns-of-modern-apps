use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::info;

#[derive(Serialize)]
pub struct LoyaltyDto {
    pub customer_id: String,
    pub current_points: f32,
    pub transactions: Vec<LoyaltyAccountTransaction>,
}

impl From<LoyaltyAccount> for LoyaltyDto {
    fn from(value: LoyaltyAccount) -> Self {
        LoyaltyDto {
            current_points: value.current_points,
            customer_id: value.customer_id,
            transactions: value.transactions,
        }
    }
}

pub(crate) struct LoyaltyAccount {
    pub(crate) customer_id: String,
    pub(crate) current_points: f32,
    pub(crate) transactions: Vec<LoyaltyAccountTransaction>,
}

impl LoyaltyAccount {
    #[tracing::instrument(name = "handle_add_transaction",skip(self))]
    pub(crate) fn add_transaction(
        &mut self,
        order_number: String,
        order_value: f32,
    ) -> Option<LoyaltyAccountTransaction> {

        let existing_transactions: Vec<&LoyaltyAccountTransaction> = self.transactions.iter().filter(|t| t.order_number == order_number).collect();

        if existing_transactions.len() > 0 {
            info!("Transaction already exists for order {}", order_number);
            return None;
        }

        let points = order_value * 0.5;
        self.current_points += points;

        let transaction = LoyaltyAccountTransaction {
            date: Utc::now(),
            order_number,
            change: points,
        };

        self.transactions.push(transaction.clone());

        Some(transaction)
    }
}

#[derive(Serialize, Clone)]
pub struct LoyaltyAccountTransaction {
    pub(crate) date: DateTime<Utc>,
    pub(crate) order_number: String,
    pub(crate) change: f32,
}

#[async_trait]
pub(crate) trait LoyaltyPoints {
    async fn new_account(&self, customer_id: String) -> LoyaltyAccount;
    async fn retrieve(&self, customer_id: &str) -> Option<LoyaltyAccount>;
    async fn add_transaction(
        &self,
        account: LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
    ) -> Result<(), ()>;
}
