use std::env;

use async_trait::async_trait;
use sqlx::PgPool;
use tracing::info;

use crate::{
    loyalty::{LoyaltyAccount, LoyaltyPoints}, LoyaltyAccountTransaction, OrderConfirmedEventHandler, RetrieveLoyaltyAccountQueryHandler
};

pub struct ApplicationAdpaters {
    pub order_confirmed_handler: OrderConfirmedEventHandler<PostgresLoyaltyPoints>,
    pub retrieve_loyalty_query_handler: RetrieveLoyaltyAccountQueryHandler<PostgresLoyaltyPoints>
}

impl ApplicationAdpaters {
    pub async fn new() -> Self {
        let database_pool = PgPool::connect(&env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        Self {
            order_confirmed_handler: OrderConfirmedEventHandler::new(PostgresLoyaltyPoints { db: database_pool.clone() }).await,
            retrieve_loyalty_query_handler: RetrieveLoyaltyAccountQueryHandler::new(PostgresLoyaltyPoints { db: database_pool }).await
        }
    }
}

pub struct PostgresLoyaltyPoints {
    db: PgPool,
}

#[async_trait]
impl LoyaltyPoints for PostgresLoyaltyPoints {
    async fn new_account(&self, customer_id: String) -> LoyaltyAccount {
        let account = LoyaltyAccount {
            customer_id,
            current_points: 0.0,
            transactions: vec![],
        };

        let _rec = sqlx::query!(
            r#"
    INSERT INTO loyalty ( customer_id, current_points )
    VALUES ( $1, $2 )
            "#,
            account.customer_id,
            account.current_points
        )
        .fetch_one(&self.db)
        .await;

        account
    }

    async fn retrieve(&self, customer_id: &str) -> Option<LoyaltyAccount> {
        info!("Searching for customer data {}", customer_id);

        let account = sqlx::query!(
            r#"
            SELECT customer_id, current_points
            FROM loyalty
            WHERE customer_id = $1
            "#,
            customer_id,
        )
        .fetch_optional(&self.db)
        .await;

        match account {
            Ok(record) => match record {
                Some(data) => Some(LoyaltyAccount {
                    customer_id: data.customer_id.unwrap(),
                    current_points: data.current_points.unwrap(),
                    transactions: vec![],
                }),
                None => None,
            },
            Err(_) => None,
        }
    }

    async fn add_transaction(
        &self,
        account: LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
    ) -> Result<(), ()> {
        info!("Opening DB transaction");

        let db_transaction = self.db.begin().await.unwrap();

        let insert_res = sqlx::query!(
            r#"
    INSERT INTO loyalty_transaction ( customer_id, date_epoch, order_number, change )
    VALUES ( $1, $2, $3, $4 )
            "#,
            account.customer_id,
            transaction.date.timestamp_millis(),
            transaction.order_number,
            transaction.change
        )
        .execute(&self.db)
        .await;

        if insert_res.is_err() {
            tracing::error!("Failure inserting transaction: {:?}", insert_res.err());
            let _ = db_transaction.rollback().await;
            return Err(());
        }

        info!("Inserted transaction");

        let update_res = sqlx::query!(
            r#"
    UPDATE loyalty
    SET current_points = $1
    WHERE customer_id = $2
            "#,
            account.current_points,
            account.customer_id
        )
        .execute(&self.db)
        .await;

        if update_res.is_err() {
            let _ = db_transaction.rollback().await;
            return Err(());
        }

        info!("Updated account");

        let _ = db_transaction.commit().await;

        info!("Committed");

        return Ok(());
    }
}
