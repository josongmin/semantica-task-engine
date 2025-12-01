// ID Provider Port (for deterministic testing)

/// ID provider interface (allows deterministic IDs in tests)
pub trait IdProvider: Send + Sync {
    /// Generate a new unique job ID
    fn generate_id(&self) -> String;
}

/// UUID v4 provider (production)
pub struct UuidProvider;

impl IdProvider for UuidProvider {
    fn generate_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
