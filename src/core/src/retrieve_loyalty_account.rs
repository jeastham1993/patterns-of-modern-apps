use std::sync::Arc;

use crate::{loyalty::LoyaltyPoints, LoyaltyDto};

pub struct RetrieveLoyaltyAccountQueryHandler<T: LoyaltyPoints + 'static> {
    loyalty_points: Arc<T>,
}

impl<T: LoyaltyPoints> RetrieveLoyaltyAccountQueryHandler<T> {
    pub async fn new(loyalty_points: Arc<T>) -> Self {
        Self { loyalty_points }
    }

    #[tracing::instrument(name = "handle_retrieve_loyalty_account", skip(self))]
    pub async fn handle(&self, customer_id: String) -> Result<LoyaltyDto, ()> {
        let loyalty_points = self
            .loyalty_points
            .retrieve(&customer_id)
            .await
            .map_err(|e| {
                tracing::error!("Failure retrieving loyalty points: {:?}", e);
                
            })?;

        Ok(loyalty_points.into())
    }
}
