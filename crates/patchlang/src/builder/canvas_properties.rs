//! Leftover-property extraction for the canvas load direction, and the matching
//! dedicated-key predicate for the emit direction.
//!
//! `InstanceLoadOutput` / `NetworkLoadOutput` surface a handful of properties as
//! dedicated fields (`location`, `protocol`, …) and carry everything else in a
//! `properties: BTreeMap<String, String>` passthrough, exactly like
//! `ChannelLabelOutput.properties` / `ConnectionLoadOutput.properties`. A key lives in
//! exactly one place: if it has a dedicated field it is NOT in the map, so a
//! load -> emit -> load round-trip re-emits the block unchanged instead of duplicating
//! the key. See `leftover_label_props` for the same rule on channel labels.
//!
//! Lives in its own file because `canvas_load.rs` is already over the 700-line ceiling
//! in `ClaudeCodeRules.md`; only the call sites belong there.

use std::collections::{BTreeMap, HashMap};

/// Instance-body keys that have a dedicated field on `InstanceLoadOutput` /
/// `InstanceEmitInput`. `ring_protocol` is read from the same property map on load
/// (`canvas_load.rs`), so it is dedicated even though `build_instance_decl` does not
/// write it — omitting it here would surface it in both the field and the map.
const INSTANCE_DEDICATED: [&str; 5] =
    ["location", "rf_band", "rf_active_channels", "iem_modes", "ring_protocol"];

/// Network-body keys that have a dedicated field on `NetworkLoadOutput`.
const NETWORK_DEDICATED: [&str; 2] = ["protocol", "label"];

/// True when `key` is a dedicated instance property (has its own field). Emit uses this
/// to skip colliding keys so a dedicated field is never written twice.
pub(crate) fn is_dedicated_instance_key(key: &str) -> bool {
    INSTANCE_DEDICATED.contains(&key)
}

/// Every instance property with no dedicated field, sorted by key.
pub(crate) fn leftover_instance_props(props: &HashMap<String, String>) -> BTreeMap<String, String> {
    leftover(props, &INSTANCE_DEDICATED)
}

/// Every network property with no dedicated field, sorted by key.
pub(crate) fn leftover_network_props(props: &HashMap<String, String>) -> BTreeMap<String, String> {
    leftover(props, &NETWORK_DEDICATED)
}

/// Shared filter: copy every entry whose key is not in `dedicated`. Collecting into a
/// `BTreeMap` is what sorts by key — `HashMap` iteration order is non-deterministic and
/// would break emit -> load -> emit idempotency.
fn leftover(props: &HashMap<String, String>, dedicated: &[&str]) -> BTreeMap<String, String> {
    props
        .iter()
        .filter(|(k, _)| !dedicated.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn instance_leftover_excludes_every_dedicated_key() {
        let props = map(&[
            ("location", "a"), ("rf_band", "b"), ("rf_active_channels", "c"),
            ("iem_modes", "d"), ("ring_protocol", "e"), ("clock_rate", "96k"),
        ]);
        let left = leftover_instance_props(&props);
        assert_eq!(left.keys().collect::<Vec<_>>(), ["clock_rate"]);
    }

    #[test]
    fn network_leftover_excludes_protocol_and_label() {
        let props = map(&[("protocol", "x"), ("label", "y"), ("domain", "0")]);
        assert_eq!(leftover_network_props(&props).keys().collect::<Vec<_>>(), ["domain"]);
    }

    #[test]
    fn leftover_is_sorted_by_key() {
        let props = map(&[("zeta", "1"), ("alpha", "2"), ("mid", "3")]);
        let keys: Vec<_> = leftover_instance_props(&props).into_keys().collect();
        assert_eq!(keys, ["alpha", "mid", "zeta"]);
    }

    #[test]
    fn dedicated_predicate_matches_the_leftover_filter() {
        for key in ["location", "rf_band", "rf_active_channels", "iem_modes", "ring_protocol"] {
            assert!(is_dedicated_instance_key(key), "{key}");
            assert!(leftover_instance_props(&map(&[(key, "v")])).is_empty(), "{key}");
        }
        assert!(!is_dedicated_instance_key("clock_rate"));
    }
}
