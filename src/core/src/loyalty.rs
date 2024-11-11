use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[cfg(any(test, feature = "mocks"))]
use mockall::{automock, predicate::*};

#[derive(Error, Debug)]
pub enum LoyaltyErrors {
    #[error("Invalid Values")]
    InvalidValues(String),
    #[error("Loyalty Account Not Found")]
    AccountNotFound(),
    #[error("Transaction Exists for Order")]
    TransactionExistsForOrder(String),
    #[error("Points Not Available")]
    PointsNotAvailable(String),
    #[error("Database Error")]
    DatabaseError(String),
}

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
pub struct LoyaltyAccount {
    customer_id: String,
    current_points: f32,
    transactions: Vec<LoyaltyAccountTransaction>,
}

impl LoyaltyAccount {
    const LOYALTY_ACCOUNT_PERCENTAGE: f32 = 0.5;

    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }

    pub fn current_points(&self) -> &f32 {
        &self.current_points
    }

    #[tracing::instrument(name = "new_loyalty_account")]
    pub fn new(customer_id: String) -> anyhow::Result<Self, LoyaltyErrors> {
        if customer_id.is_empty() {
            return Err(LoyaltyErrors::InvalidValues(
                "CustomerID cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            customer_id,
            current_points: 0.00,
            transactions: vec![],
        })
    }

    pub fn from(
        customer_id: String,
        current_points: f32,
        transactions: Vec<LoyaltyAccountTransaction>,
    ) -> anyhow::Result<Self, LoyaltyErrors> {
        if customer_id.is_empty() {
            return Err(LoyaltyErrors::InvalidValues(
                "CustomerID cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            customer_id,
            current_points,
            transactions,
        })
    }

    #[tracing::instrument(name = "handle_add_transaction", skip(self))]
    pub(crate) fn add_transaction(
        &mut self,
        order_number: String,
        order_value: f32,
    ) -> anyhow::Result<LoyaltyAccountTransaction, LoyaltyErrors> {
        let existing_transactions: Vec<&LoyaltyAccountTransaction> = self
            .transactions
            .iter()
            .filter(|t| t.order_number == order_number)
            .collect();

        if !existing_transactions.is_empty() {
            info!("Transaction already exists for order {}", order_number);
            return Err(LoyaltyErrors::TransactionExistsForOrder(format!(
                "Transaction already exists for order {}",
                order_number
            )));
        }

        let points = order_value * Self::LOYALTY_ACCOUNT_PERCENTAGE;
        self.current_points += points;

        let transaction = LoyaltyAccountTransaction {
            date: Utc::now(),
            order_number,
            change: points,
        };

        self.transactions.push(transaction.clone());

        Ok(transaction)
    }

    pub(crate) fn spend_points(
        &mut self,
        order_number: &str,
        spend: &f32,
    ) -> Result<LoyaltyAccountTransaction, LoyaltyErrors> {
        let new_points_total = self.current_points - spend;

        if new_points_total < 0.0 {
            return Err(LoyaltyErrors::PointsNotAvailable(
                "Current points not enough to cover this transaction".to_string(),
            ));
        }

        let existing_transactions: Vec<&LoyaltyAccountTransaction> = self
            .transactions
            .iter()
            .filter(|t| t.order_number == order_number)
            .collect();

        if !existing_transactions.is_empty() {
            return Err(LoyaltyErrors::TransactionExistsForOrder(format!(
                "Transaction already exists for order {}",
                order_number
            )));
        }

        self.current_points = new_points_total;

        let transaction = LoyaltyAccountTransaction {
            date: Utc::now(),
            order_number: order_number.to_string(),
            change: -spend,
        };

        self.transactions.push(transaction.clone());

        Ok(transaction)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LoyaltyAccountTransaction {
    pub(crate) date: DateTime<Utc>,
    pub(crate) order_number: String,
    pub(crate) change: f32,
}

impl LoyaltyAccountTransaction {
    pub fn new(date: DateTime<Utc>, order_number: String, change: f32) -> Self {
        Self {
            date,
            order_number,
            change
        }
    }

    pub fn date(&self) -> DateTime<Utc> {
        self.date
    }
    pub fn order_number(&self) -> String {
        self.order_number.clone()
    }
    pub fn change(&self) -> f32 {
        self.change
    }
}

#[cfg_attr(any(test, feature = "mocks"), automock)]
#[async_trait]
pub trait LoyaltyPoints {
    async fn new_account(
        &self,
        customer_id: String,
    ) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors>;
    async fn retrieve(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors>;
    async fn add_transaction(
        &self,
        account: &LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
    ) -> anyhow::Result<(), LoyaltyErrors>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_loyalty_account() {
        let test_customer_id = "test-id";
        let account = LoyaltyAccount::new(test_customer_id.to_string()).unwrap();

        assert_eq!(account.customer_id, test_customer_id);
        assert_eq!(account.current_points, 0.00);
        assert_eq!(account.transactions.len(), 0);
    }

    #[test]
    fn can_create_loyalty_account_and_add_transaction() {
        let test_customer_id = "test-id";
        let mut account = LoyaltyAccount::new(test_customer_id.to_string()).unwrap();
        let _ = account.add_transaction("ORD567".to_string(), 100.00);

        assert_eq!(account.current_points, 50.00);
        assert_eq!(account.transactions.len(), 1);
    }

    #[test]
    fn can_create_loyalty_account_and_spend_points_when_points_are_available() {
        let test_customer_id = "test-id";
        let mut account = LoyaltyAccount::new(test_customer_id.to_string()).unwrap();
        let _ = account.add_transaction("ORD567".to_string(), 100.00);

        let _ = account.spend_points("ORD789", &10.0);

        assert_eq!(account.current_points, 40.00);
        assert_eq!(account.transactions.len(), 2);
    }

    #[test]
    fn can_create_loyalty_account_and_add_same_transaction_should_not_add_points() {
        let test_customer_id = "test-id";
        let mut account = LoyaltyAccount::new(test_customer_id.to_string()).unwrap();
        let _ = account.add_transaction("ORD567".to_string(), 100.00);
        let _ = account.add_transaction("ORD567".to_string(), 100.00);

        assert_eq!(account.current_points, 50.00);
        assert_eq!(account.transactions.len(), 1);
    }

    #[test]
    fn can_create_loyalty_account_from_parts() {
        let test_customer_id = "test-id";
        let test_points_total = 10.0;
        let transactions = vec![];

        let account = LoyaltyAccount::from(
            test_customer_id.to_string(),
            test_points_total,
            transactions.clone(),
        )
        .unwrap();

        assert_eq!(account.customer_id, test_customer_id);
        assert_eq!(account.current_points, test_points_total);
        assert_eq!(account.transactions.len(), transactions.len());
    }

    #[test]
    fn can_create_loyalty_account_from_parts_and_add_transactions() {
        let test_customer_id = "test-id";
        let test_points_total = 10.0;
        let transactions = vec![];

        let mut account = LoyaltyAccount::from(
            test_customer_id.to_string(),
            test_points_total,
            transactions.clone(),
        )
        .unwrap();

        let _ = account.add_transaction("ORD567".to_string(), 100.00);

        assert_eq!(account.current_points, 60.00);
        assert_eq!(account.transactions.len(), 1);
    }
}
