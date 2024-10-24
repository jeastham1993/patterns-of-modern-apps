use std::env;

use async_trait::async_trait;
use chrono::DateTime;
use sqlx::PgPool;
use tracing::info;

use crate::{
    loyalty::{LoyaltyAccount, LoyaltyErrors, LoyaltyPoints},
    LoyaltyAccountTransaction, OrderConfirmedEventHandler, RetrieveLoyaltyAccountQueryHandler,
};

pub struct ApplicationAdpaters {
    pub order_confirmed_handler: OrderConfirmedEventHandler<PostgresLoyaltyPoints>,
    pub retrieve_loyalty_query_handler: RetrieveLoyaltyAccountQueryHandler<PostgresLoyaltyPoints>,
}

impl ApplicationAdpaters {
    #[tracing::instrument(name = "new_application_adapters")]
    pub async fn new() -> Self {
        let database_pool = PgPool::connect(&env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        Self {
            order_confirmed_handler: OrderConfirmedEventHandler::new(PostgresLoyaltyPoints {
                db: database_pool.clone(),
            })
            .await,
            retrieve_loyalty_query_handler: RetrieveLoyaltyAccountQueryHandler::new(
                PostgresLoyaltyPoints { db: database_pool },
            )
            .await,
        }
    }
}

pub struct PostgresLoyaltyPoints {
    db: PgPool,
}

#[async_trait]
impl LoyaltyPoints for PostgresLoyaltyPoints {
    #[tracing::instrument(name = "db_new_account", skip(self))]
    async fn new_account(
        &self,
        customer_id: String,
    ) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
        let account = LoyaltyAccount::new(customer_id)?;

        let _rec = sqlx::query!(
            r#"
    INSERT INTO loyalty ( customer_id, current_points )
    VALUES ( $1, $2 )
            "#,
            account.customer_id(),
            account.current_points()
        )
        .fetch_one(&self.db)
        .await;

        Ok(account)
    }

    #[tracing::instrument(name = "db_retrieve", skip(self))]
    async fn retrieve(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
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
                Some(data) => {
                    let transactions = sqlx::query!(
                        r#"
                        SELECT customer_id, date_epoch, order_number, change
                        FROM loyalty_transaction
                        WHERE customer_id = $1
                        "#,
                        customer_id,
                    )
                    .fetch_all(&self.db)
                    .await;

                    let loyalty_transactions = match transactions {
                        Ok(rows) => rows
                            .iter()
                            .map(|row| LoyaltyAccountTransaction {
                                change: row.change.unwrap(),
                                order_number: row.order_number.clone().unwrap(),
                                date: DateTime::from_timestamp_millis(row.date_epoch.unwrap())
                                    .unwrap(),
                            })
                            .collect(),
                        Err(_) => vec![],
                    };

                    let found_account = LoyaltyAccount::from(
                        data.customer_id.unwrap(),
                        data.current_points.unwrap(),
                        loyalty_transactions,
                    )?;

                    Ok(found_account)
                }
                None => Err(LoyaltyErrors::AccountNotFound()),
            },
            Err(e) => Err(LoyaltyErrors::DatabaseError(format!(
                "Database Error: {:?}",
                e
            ))),
        }
    }

    #[tracing::instrument(name = "db_add_transaction", skip(self, account, transaction))]
    async fn add_transaction(
        &self,
        account: LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
    ) -> anyhow::Result<(), LoyaltyErrors> {
        info!("Opening DB transaction");

        let db_transaction = self.db.begin().await.unwrap();

        let insert_res = sqlx::query!(
            r#"
    INSERT INTO loyalty_transaction ( customer_id, date_epoch, order_number, change )
    VALUES ( $1, $2, $3, $4 )
            "#,
            account.customer_id(),
            transaction.date.timestamp_millis(),
            transaction.order_number,
            transaction.change
        )
        .execute(&self.db)
        .await;

        if insert_res.is_err() {
            tracing::error!("Failure inserting transaction: {:?}", insert_res.err());
            let _ = db_transaction.rollback().await;
            return Err(LoyaltyErrors::DatabaseError(
                "Failure inserting transaction".to_string(),
            ));
        }

        info!("Inserted transaction");

        let update_res = sqlx::query!(
            r#"
    UPDATE loyalty
    SET current_points = $1
    WHERE customer_id = $2
            "#,
            account.current_points(),
            account.customer_id()
        )
        .execute(&self.db)
        .await;

        if update_res.is_err() {
            let _ = db_transaction.rollback().await;
            return Err(LoyaltyErrors::DatabaseError(
                "Failure updating account".to_string(),
            ));
        }

        info!("Updated account");

        let _ = db_transaction.commit().await;

        info!("Committed");

        return Ok(());
    }
}
