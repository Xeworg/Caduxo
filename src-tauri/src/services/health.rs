//! Health-check service.
use crate::state::AppState;

pub struct HealthService;

impl HealthService {
    /// Verifies the database is reachable and returns its status.
    pub async fn check(state: &AppState) -> Result<bool, sqlx::Error> {
        state.ping_db().await
    }
}
