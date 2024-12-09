use chrono::{DateTime, Duration, Utc};

use crate::entity::Entity;

const NEARLY_EXPIRING_ALLOWANCE: Duration = Duration::weeks(1);

pub enum EntityExpiration {
    NotExpiring,
    Expiring,
    Expired,
}

impl EntityExpiration {
    pub fn for_expiration_dt(expiration_dt: DateTime<Utc>) -> Option<Self> {
        let now = Utc::now();

        let expiration = if expiration_dt < now {
            EntityExpiration::Expired
        } else if expiration_dt - now < NEARLY_EXPIRING_ALLOWANCE {
            EntityExpiration::Expiring
        } else {
            EntityExpiration::NotExpiring
        };
        Some(expiration)
    }
}

pub trait EntityExpirationEntityExt {
    fn expiration(&self) -> Option<EntityExpiration>;
}

impl EntityExpirationEntityExt for Entity {
    fn expiration(&self) -> Option<EntityExpiration> {
        let expiration_dt = *self.data().expiration_dt()?;
        EntityExpiration::for_expiration_dt(expiration_dt)
    }
}
