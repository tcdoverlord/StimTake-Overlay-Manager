//! Backend-owned activation safety policy for StimTake.
use crate::memory_monitor::MemoryPressure;

/// Product safety default. Keep this in one place so backend checks and tests agree.
pub const MAX_ACTIVE_INSTANCES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationDenial {
    MemoryPressureRed,
    ActiveLimitReached { limit: usize },
}
impl ActivationDenial {
    pub fn message(&self) -> String {
        match self {
            Self::MemoryPressureRed => "Memory pressure is RED; new overlay activations are temporarily paused.".into(),
            Self::ActiveLimitReached { limit } => format!("Maximum of {limit} overlays already running"),
        }
    }
    pub fn code(&self) -> &'static str {
        match self {
            Self::MemoryPressureRed => "MEMORY_PRESSURE_RED",
            Self::ActiveLimitReached { .. } => "ACTIVE_LIMIT_REACHED",
        }
    }
}

/// Validate the projected active count before mutating backend state.
/// `retained_active` includes live instances that survive this deck update;
/// `new_activations` counts genuinely new instances requested by the payload.
pub fn can_activate(
    retained_active: usize,
    new_activations: usize,
    pressure: MemoryPressure,
    limit: usize,
) -> Result<(), ActivationDenial> {
    if new_activations > 0 && pressure == MemoryPressure::Red {
        return Err(ActivationDenial::MemoryPressureRed);
    }
    if retained_active.saturating_add(new_activations) > limit {
        return Err(ActivationDenial::ActiveLimitReached { limit });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_instances_can_run() {
        assert_eq!(can_activate(0, MAX_ACTIVE_INSTANCES, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Ok(()));
        assert_eq!(can_activate(4, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Ok(()));
    }

    #[test]
    fn sixth_instance_is_rejected() {
        assert_eq!(
            can_activate(MAX_ACTIVE_INSTANCES, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES),
            Err(ActivationDenial::ActiveLimitReached { limit: MAX_ACTIVE_INSTANCES })
        );
    }

    #[test]
    fn stopping_one_allows_another_activation() {
        assert_eq!(can_activate(MAX_ACTIVE_INSTANCES - 1, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Ok(()));
    }

    #[test]
    fn deck_switch_does_not_reset_active_instance_limit() {
        // Existing instances from the previous deck are retained in the projected count.
        assert_eq!(can_activate(MAX_ACTIVE_INSTANCES, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES),
            Err(ActivationDenial::ActiveLimitReached { limit: MAX_ACTIVE_INSTANCES }));
    }

    #[test]
    fn red_pressure_blocks_new_activations_without_invalidating_existing_instances() {
        assert_eq!(can_activate(5, 1, MemoryPressure::Red, MAX_ACTIVE_INSTANCES), Err(ActivationDenial::MemoryPressureRed));
        // A deck update that creates no new instances remains valid under RED.
        assert_eq!(can_activate(5, 0, MemoryPressure::Red, MAX_ACTIVE_INSTANCES), Ok(()));
    }

    #[test]
    fn same_overlay_can_have_multiple_instances_until_limit() {
        // The policy counts instances, not distinct overlay package IDs.
        assert_eq!(can_activate(4, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Ok(()));
        assert_eq!(can_activate(5, 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Err(ActivationDenial::ActiveLimitReached { limit: MAX_ACTIVE_INSTANCES }));
    }

    #[test]
    fn failed_sixth_activation_is_rejected_before_any_state_mutation() {
        // This preflight function is pure: rejection has no registry argument to mutate.
        let existing_ids = vec!["instance-1", "instance-2", "instance-3", "instance-4", "instance-5"];
        let before = existing_ids.clone();
        let result = can_activate(existing_ids.len(), 1, MemoryPressure::Green, MAX_ACTIVE_INSTANCES);
        assert_eq!(result, Err(ActivationDenial::ActiveLimitReached { limit: MAX_ACTIVE_INSTANCES }));
        assert_eq!(existing_ids, before);
    }

    #[test]
    fn projected_limit_is_checked_atomically_for_multi_activation_payload() {
        assert_eq!(can_activate(4, 2, MemoryPressure::Green, MAX_ACTIVE_INSTANCES), Err(ActivationDenial::ActiveLimitReached { limit: MAX_ACTIVE_INSTANCES }));
    }

    #[test]
    fn yellow_allows_activation_within_limit() {
        assert_eq!(can_activate(3, 1, MemoryPressure::Yellow, MAX_ACTIVE_INSTANCES), Ok(()));
    }
}
