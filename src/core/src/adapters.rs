use std::{env, time::Duration};

use async_trait::async_trait;
use chrono::DateTime;
use momento::{cache::GetResponse, CacheClient, CredentialProvider};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::{
    loyalty::{LoyaltyAccount, LoyaltyErrors, LoyaltyPoints},
    LoyaltyAccountTransaction,
};

pub struct ApplicationAdapters<T: LoyaltyPoints + Send + Sync> {
    pub loyalty_points: T,
}

impl<T: LoyaltyPoints + Send + Sync> ApplicationAdapters<T> {
    #[tracing::instrument(name = "new_application_adapters", skip(loyalty))]
    pub async fn new(loyalty: T) -> Self {
        Self {
            loyalty_points: loyalty,
        }
    }
}

pub struct PostgresLoyaltyPoints {
    db: PgPool,
    cache_client: Option<CacheClient>,
    cache_name: String,
}

impl PostgresLoyaltyPoints {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let db_url = &env::var("DATABASE_URL")?;
        let database_pool = PgPool::connect(db_url).await?;

        let (cache_client, cache_name) = PostgresLoyaltyPoints::configure_cache_client().await;

        Ok(Self {
            db: database_pool,
            cache_client,
            cache_name,
        })
    }

    async fn configure_cache_client() -> (Option<CacheClient>, String) {
        let key = env::var("MOMENTO_API_KEY");
        let cache_name = env::var("CACHE_NAME").unwrap_or(String::from(""));

        let cache_client = match key {
            Ok(key) => {
                let credential_provider = CredentialProvider::from_string(key);

                match credential_provider {
                    Ok(credential_provider) => {
                        let client = CacheClient::builder()
                            .default_ttl(Duration::from_secs(600))
                            .configuration(momento::cache::configurations::Lambda::latest())
                            .credential_provider(credential_provider)
                            .build();

                        match client {
                            Ok(client) => {
                                if cache_name.len() <= 0 {
                                    None
                                } else {
                                    Some(client)
                                }
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };

        (cache_client, cache_name)
    }

    #[tracing::instrument(name = "cache_get", skip(self))]
    async fn cache_get(&self, customer_id: &str) -> Result<String, ()> {
        match &self.cache_client {
            Some(cache_client) => {
                let get_result = cache_client
                    .get(&self.cache_name, customer_id)
                    .await
                    .map_err(|_e| LoyaltyErrors::AccountNotFound());

                match get_result {
                    Ok(res) => match res {
                        GetResponse::Hit { value } => {
                            info!("Cache hit");
                            let val: String =
                                value.try_into().expect("Cached value to match string");

                            Ok(val)
                        }
                        GetResponse::Miss => {
                            warn!("Cache miss");
                            Err(())
                        }
                    },
                    Err(_) => Err(()),
                }
            }
            None => Err(()),
        }
    }

    #[tracing::instrument(name = "cache_put", skip(self, account))]
    async fn cache_put(&self, account: &LoyaltyAccount) {
        match &self.cache_client {
            Some(cache_client) => {
                let cache_data = serde_json::to_string(account).unwrap_or(String::from(""));

                match cache_client
                    .set(&self.cache_name, account.customer_id(), cache_data)
                    .await
                {
                    Ok(_) => info!("Successfully cached"),
                    Err(e) => tracing::error!("Error: {}", e),
                }
            }
            None => {}
        };
    }

    #[tracing::instrument(name = "db_get", skip(self))]
    async fn db_get(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
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

                    let _ = &self.cache_put(&found_account).await;

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

    #[tracing::instrument(name = "retrieve", skip(self))]
    async fn retrieve(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
        info!("Searching for customer data {}", customer_id);

        let cache_val = &self.cache_get(customer_id).await;

        match cache_val {
            Ok(account) => {
                let parsed_account = serde_json::from_str(account).unwrap();
                Ok(parsed_account)
            }
            Err(_) => self.db_get(customer_id).await,
        }
    }

    #[tracing::instrument(name = "db_add_transaction", skip(self, account, transaction))]
    async fn add_transaction(
        &self,
        account: &LoyaltyAccount,
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

        let _ = &self.cache_put(account).await;

        info!("Committed");

        return Ok(());
    }
}
