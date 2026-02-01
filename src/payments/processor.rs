use crate::payments::cache::SubscriptionCache;
use crate::payments::types::PaymentEvent;
use tracing::info;

/// Process a detected payment event.
pub async fn process_payment(event: PaymentEvent, cache: &SubscriptionCache) {
    info!(
        "Processing payment: User {:?} paid {} for Tier {}",
        event.user, event.amount, event.tier_id
    );

    // Calculate expiry (e.g. 30 days from now)
    // In real system, might depend on amount or plan
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expiry = now + 30 * 24 * 3600; // 30 days default

    cache.update_subscription(event.user, event.tier_id, expiry);
    info!("Updated subscription for user {:?}", event.user);
    
    // Attempt save? (optional, maybe too frequent)
    // cache.save_to_file().ok();
}
