//! Cache eviction safety boundary for StimTake.
//!
//! Every eviction entry point funnels through `can_evict`. The active registry
//! maps each live overlay instance to every resource URL that instance needs.
//! Removing an instance releases its protection only when no other live instance
//! still references the resource. Imported files on disk are never deleted here.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier { Hot, Warm, Cold }

#[derive(Debug, Clone)]
pub struct CachedResource {
    pub id: String,
    pub last_used_unix: u64,
    pub estimated_bytes: u64,
    pub tier: CacheTier,
}

#[derive(Debug, Default, Clone)]
pub struct ActiveOverlayRegistry {
    instance_resources: HashMap<String, HashSet<String>>,
}
impl ActiveOverlayRegistry {
    pub fn register_instance<I, S>(&mut self, instance_id: String, resources: I)
    where I: IntoIterator<Item = S>, S: Into<String> {
        self.instance_resources.insert(instance_id, resources.into_iter().map(Into::into).collect());
    }

    pub fn unregister_instance(&mut self, instance_id: &str) {
        self.instance_resources.remove(instance_id);
    }

    pub fn resource_is_required_by_live_instance(&self, resource_id: &str) -> bool {
        self.instance_resources.values().any(|resources| resources.contains(resource_id))
    }

    pub fn active_instance_count(&self) -> usize { self.instance_resources.len() }

    pub fn protected_resource_count(&self) -> usize {
        self.instance_resources.values().flat_map(|set| set.iter()).collect::<HashSet<_>>().len()
    }
}

#[derive(Debug, Default)]
pub struct CacheManager {
    resources: HashMap<String, CachedResource>,
}
impl CacheManager {
    pub fn insert(&mut self, resource: CachedResource) { self.resources.insert(resource.id.clone(), resource); }
    pub fn contains(&self, id: &str) -> bool { self.resources.contains_key(id) }
    pub fn len(&self) -> usize { self.resources.len() }
    pub fn is_empty(&self) -> bool { self.resources.is_empty() }

    /// The one safety gate used by every cleanup path.
    pub fn can_evict(&self, resource: &CachedResource, active: &ActiveOverlayRegistry) -> bool {
        !active.resource_is_required_by_live_instance(&resource.id)
    }

    fn evict_if_allowed(&mut self, id: &str, active: &ActiveOverlayRegistry) -> bool {
        let allowed = self.resources.get(id).map(|r| self.can_evict(r, active)).unwrap_or(false);
        if allowed { self.resources.remove(id).is_some() } else { false }
    }

    /// LRU eviction: oldest access first, but never a resource used by a live instance.
    pub fn evict_lru_until_below(&mut self, target_bytes: u64, active: &ActiveOverlayRegistry) -> Vec<String> {
        let mut candidates: Vec<(u64, String)> = self.resources.values()
            .filter(|r| self.can_evict(r, active))
            .map(|r| (r.last_used_unix, r.id.clone())).collect();
        candidates.sort_by_key(|(last_used, _)| *last_used);
        let mut total: u64 = self.resources.values().map(|r| r.estimated_bytes).sum();
        let mut removed = Vec::new();
        for (_, id) in candidates {
            if total <= target_bytes { break; }
            if let Some(bytes) = self.resources.get(&id).map(|r| r.estimated_bytes) {
                if self.evict_if_allowed(&id, active) { total = total.saturating_sub(bytes); removed.push(id); }
            }
        }
        removed
    }

    /// RED-pressure cleanup: discard non-active warm/cold resources, never hot resources
    /// that are needed by a live overlay. Selected-deck protection is layered above this.
    pub fn evict_non_active_lru(&mut self, active: &ActiveOverlayRegistry) -> Vec<String> {
        let mut candidates: Vec<(u64, String)> = self.resources.values()
            .filter(|r| r.tier != CacheTier::Hot && self.can_evict(r, active))
            .map(|r| (r.last_used_unix, r.id.clone())).collect();
        candidates.sort_by_key(|(last_used, _)| *last_used);
        candidates.into_iter().filter_map(|(_, id)| self.evict_if_allowed(&id, active).then_some(id)).collect()
    }

    /// Expiration cleanup must use the same gate as LRU cleanup.
    pub fn evict_expired(&mut self, expired_ids: &[String], active: &ActiveOverlayRegistry) -> Vec<String> {
        expired_ids.iter().filter_map(|id| self.evict_if_allowed(id, active).then_some(id.clone())).collect()
    }

    /// Manual cleanup is also forbidden from evicting live-overlay resources.
    pub fn manual_evict(&mut self, id: &str, active: &ActiveOverlayRegistry) -> bool {
        self.evict_if_allowed(id, active)
    }

    /// Deck switching can age resources, but cannot evict resources still required by active instances.
    pub fn on_deck_switch_cleanup(&mut self, active: &ActiveOverlayRegistry) -> Vec<String> {
        let mut candidates: Vec<(u64, String)> = self.resources.values()
            .filter(|r| r.tier == CacheTier::Warm && self.can_evict(r, active))
            .map(|r| (r.last_used_unix, r.id.clone())).collect();
        candidates.sort_by_key(|(last_used, _)| *last_used);
        candidates.into_iter().filter_map(|(_, id)| self.evict_if_allowed(&id, active).then_some(id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resource(id: &str, tier: CacheTier, last_used_unix: u64) -> CachedResource {
        CachedResource { id: id.into(), last_used_unix, estimated_bytes: 100, tier }
    }

    #[test]
    fn active_resource_cannot_be_evicted_by_any_cleanup_path() {
        let mut active = ActiveOverlayRegistry::default();
        active.register_instance("ghost-live".into(), ["ghost.html", "shared.png"]);
        let mut cache = CacheManager::default();
        cache.insert(resource("ghost.html", CacheTier::Warm, 1));
        cache.insert(resource("shared.png", CacheTier::Cold, 2));
        assert!(!cache.manual_evict("ghost.html", &active));
        assert!(cache.evict_expired(&["shared.png".into()], &active).is_empty());
        assert!(cache.evict_lru_until_below(0, &active).is_empty());
        assert!(cache.on_deck_switch_cleanup(&active).is_empty());
        assert!(cache.evict_non_active_lru(&active).is_empty());
        assert!(cache.contains("ghost.html"));
        assert!(cache.contains("shared.png"));
    }

    #[test]
    fn shared_resource_stays_protected_until_last_instance_stops() {
        let mut active = ActiveOverlayRegistry::default();
        active.register_instance("one".into(), ["shared.js"]);
        active.register_instance("two".into(), ["shared.js"]);
        let mut cache = CacheManager::default();
        cache.insert(resource("shared.js", CacheTier::Cold, 1));
        active.unregister_instance("one");
        assert!(!cache.manual_evict("shared.js", &active));
        active.unregister_instance("two");
        assert!(cache.manual_evict("shared.js", &active));
    }

    #[test]
    fn lru_removes_oldest_evictable_resources_first_and_keeps_active() {
        let mut active = ActiveOverlayRegistry::default();
        active.register_instance("live".into(), ["live.html"]);
        let mut cache = CacheManager::default();
        cache.insert(resource("live.html", CacheTier::Warm, 0));
        cache.insert(resource("old.css", CacheTier::Warm, 1));
        cache.insert(resource("new.css", CacheTier::Warm, 2));
        let removed = cache.evict_lru_until_below(200, &active);
        assert_eq!(removed, vec!["old.css".to_string()]);
        assert!(cache.contains("live.html"));
        assert!(!cache.contains("old.css"));
        assert!(cache.contains("new.css"));
    }

    #[test]
    fn red_cleanup_does_not_evict_hot_or_active_resources() {
        let mut active = ActiveOverlayRegistry::default();
        active.register_instance("live".into(), ["live.html"]);
        let mut cache = CacheManager::default();
        cache.insert(resource("live.html", CacheTier::Hot, 0));
        cache.insert(resource("selected-hot.css", CacheTier::Hot, 1));
        cache.insert(resource("old-warm.css", CacheTier::Warm, 2));
        let removed = cache.evict_non_active_lru(&active);
        assert_eq!(removed, vec!["old-warm.css".to_string()]);
        assert!(cache.contains("live.html"));
        assert!(cache.contains("selected-hot.css"));
    }
}
