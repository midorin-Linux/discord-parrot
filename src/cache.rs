use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

struct Cache {
    sub_cache: Arc<RwLock<HashMap<u64, u64, Vec<u64>>>>,
}

impl Cache {
    pub async fn is_subscribed(&self, channel_id: u64) -> bool {
        let sub_cache = self.sub_cache.read().await;
        !sub_cache.is_empty()
    }
}