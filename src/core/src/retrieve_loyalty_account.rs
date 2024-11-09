use crate::{loyalty::LoyaltyPoints, LoyaltyDto};

pub struct RetrieveLoyaltyAccountQueryHandler;

impl RetrieveLoyaltyAccountQueryHandler {
    #[tracing::instrument(name = "handle_retrieve_loyalty_account", skip(loyalty_points))]
    pub async fn handle<T: LoyaltyPoints>(loyalty_points: &T, customer_id: String) -> Result<LoyaltyDto, ()> {
        let loyalty_points = loyalty_points
            .retrieve(&customer_id)
            .await
            .map_err(|e| {
                tracing::error!("Failure retrieving loyalty points: {:?}", e);
                
            })?;

        Ok(loyalty_points.into())
    }
}
