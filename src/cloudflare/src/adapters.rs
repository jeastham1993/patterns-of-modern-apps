use async_trait::async_trait;
use chrono::DateTime;
use loyalty_core::{LoyaltyAccount, LoyaltyAccountTransaction, LoyaltyErrors, LoyaltyPoints};
use serde::Deserialize;
use wasm_bindgen_futures::wasm_bindgen::JsValue;
use worker::D1Database;

pub struct D1DataAccessLayer {
    db: D1Database,
}

impl D1DataAccessLayer {
    pub(crate) async fn new(db: D1Database) -> Self {
        Self { db }
    }
}

#[derive(Deserialize)]
struct LoyaltyAccountRow {
    customer_id: String,
    current_points: f32,
}

#[derive(Deserialize)]
struct LoyaltyTransactionRow {
    date_epoch: i32,
    order_number: String,
    change: f32,
}

#[worker::send]
async fn insert_new_account_to_db(value: &D1DataAccessLayer, account: &LoyaltyAccount) {
    let _ = value
        .db
        .prepare("INSERT INTO loyalty ( customer_id, current_points ) VALUES ( ?1, ?2 )")
        .bind(&[
            JsValue::from(account.customer_id()),
            JsValue::from(account.current_points().clone()),
        ])
        .unwrap()
        .run()
        .await;
}

#[worker::send]
async fn retrieve_from_db(
    value: &D1DataAccessLayer,
    customer_id: &str,
) -> Option<LoyaltyAccountRow> {
    let res = value
        .db
        .prepare("SELECT customer_id, current_points FROM loyalty WHERE customer_id = ?1")
        .bind(&[JsValue::from(customer_id)])
        .unwrap()
        .first::<LoyaltyAccountRow>(None)
        .await;

    match res {
        Ok(account) => account,
        Err(e) => {
            tracing::error!("Failure querying database '{:?}", e);

            None
        }
    }
}

#[worker::send]
async fn retrieve_transactions_from_db(
    value: &D1DataAccessLayer,
    customer_id: &str,
) -> Vec<LoyaltyAccountTransaction> {
    let res = value
        .db
        .prepare("SELECT date_epoch, order_number, change FROM loyalty_transaction WHERE customer_id = ?1")
        .bind(&[JsValue::from(customer_id)])
        .unwrap()
        .all()
        .await;

    match res {
        Ok(results) => {
            let transactions = results.results::<LoyaltyTransactionRow>();

            match transactions {
                Ok(transaction) => transaction
                    .iter()
                    .map(|transaction| {
                        LoyaltyAccountTransaction::new(
                            DateTime::from_timestamp_millis(transaction.date_epoch as i64).unwrap(),
                            transaction.order_number.clone(),
                            transaction.change.clone(),
                        )
                    })
                    .collect(),
                Err(e) => {
                    tracing::error!(
                        "Failure parsing LoyaltyAccountTransaction from db row '{:?}",
                        e
                    );

                    vec![]
                }
            }
        }
        Err(e) => {
            tracing::error!("Failure querying database '{:?}", e);

            vec![]
        }
    }
}

#[worker::send]
async fn add_transaction_to_db(
    value: &D1DataAccessLayer,
    account: &LoyaltyAccount,
    transaction: &LoyaltyAccountTransaction,
) {
    let timestamp_millis = transaction.date().timestamp_millis() as i32;

    let _ = value
        .db
        .prepare("INSERT INTO loyalty_transaction (customer_id, date_epoch, order_number, change) VALUES (?1, ?2, ?3, ?4)")
        .bind(&[
            JsValue::from(account.customer_id()),
            JsValue::from(timestamp_millis),
            JsValue::from(transaction.order_number()),
            JsValue::from(transaction.change()),
        ])
        .unwrap()
        .run()
        .await;
}

#[worker::send]
async fn update_total_points_in_db(value: &D1DataAccessLayer, account: &LoyaltyAccount) {
    let _ = value
        .db
        .prepare("UPDATE loyalty SET current_points = $1 WHERE customer_id = $2")
        .bind(&[
            JsValue::from(account.current_points().clone()),
            JsValue::from(account.customer_id()),
        ])
        .unwrap()
        .run()
        .await;
}

#[async_trait]
impl LoyaltyPoints for D1DataAccessLayer {
    async fn new_account(
        &self,
        customer_id: String,
    ) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
        let account = LoyaltyAccount::new(customer_id)?;

        insert_new_account_to_db(&self, &account).await;

        Ok(account)
    }

    async fn retrieve(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors> {
        let account = retrieve_from_db(&self, &customer_id).await;

        match account {
            Some(account) => {
                let transactions = retrieve_transactions_from_db(&self, &customer_id).await;

                Ok(
                    LoyaltyAccount::from(account.customer_id, account.current_points, transactions)
                        .unwrap(),
                )
            }
            None => Err(LoyaltyErrors::AccountNotFound()),
        }
    }

    async fn add_transaction(
        &self,
        account: &LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
    ) -> anyhow::Result<(), LoyaltyErrors> {
        add_transaction_to_db(&self, account, &transaction).await;

        update_total_points_in_db(&self, account).await;

        Ok(())
    }
}
