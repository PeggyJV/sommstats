/// A simple cache that has an expiration timestamp. Useful for lazy loading data on
/// user query as opposed to polling on some cadence.
pub struct ExpiringCache<T> {
    expiration: std::time::SystemTime,
    pub data: T,
}

impl<T: Default> ExpiringCache<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_expired(&self) -> bool {
        self.expiration <= std::time::SystemTime::now()
    }

    pub fn set_expiration(&mut self, expiration: std::time::Duration) {
        self.expiration = std::time::SystemTime::now() + expiration;
    }
}

impl<T: Default> Default for ExpiringCache<T> {
    fn default() -> Self {
        Self {
            expiration: std::time::SystemTime::now(),
            data: T::default(),
        }
    }
}
