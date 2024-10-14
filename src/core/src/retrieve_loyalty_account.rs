use crate::{loyalty::LoyaltyPoints, LoyaltyDto};

pub struct RetrieveLoyaltyAccountQueryHandler<T: LoyaltyPoints> {
    loyalty_points: T,
}

impl<T: LoyaltyPoints> RetrieveLoyaltyAccountQueryHandler<T> {
    pub async fn new(loyalty_points: T) -> Self {
        Self { loyalty_points }
    }

    pub async fn handle(&self, customer_id: String) -> Result<LoyaltyDto, ()> {
        let loyalty_points = self.loyalty_points.retrieve(&customer_id).await;

        match loyalty_points {
            Some(account) => Ok(account.into()),
            None => Err(()),
        }
        
    }
}
