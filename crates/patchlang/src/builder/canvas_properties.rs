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
