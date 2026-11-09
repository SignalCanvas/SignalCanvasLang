//! Edge cases for the instance/network property passthrough: which keys stay out of the
//! map, networks with several keys, and keyword keys the parser cannot carry.

use crate::builder::canvas_load::load_from_patch;

/// Every instance key with a dedicated field must stay out of `properties`; only the
/// custom key is leftover. Guards the "a key lives in exactly one place" rule.
#[test]
fn instance_dedicated_keys_never_appear_in_properties() {
    let src = r#"
template T { meta { model: "T" } }
instance D is T {
  location: "Rack A"
  rf_band: "G57"
  rf_active_channels: 4
  iem_modes: "stereo"
  ring_protocol: "OptoCore"
  clock_source: "internal"
}
"#;
    let loaded = load_from_patch(src, "").expect("load");
    let props = &loaded.instances[0].properties;
    assert_eq!(props.len(), 1, "only the custom key is leftover: {props:?}");
    assert_eq!(props.get("clock_source").map(String::as_str), Some("internal"));
}

/// A network with `label` and extra keys: label has its own field, extras are sorted.
#[test]
fn network_label_and_extra_properties_together() {
    let src = "network N {\n  protocol: \"PTPv2\"\n  label: \"Main\"\n  priority2: \"128\"\n  domain: \"0\"\n}\n";
    let loaded = load_from_patch(src, "").expect("load");
    let net = &loaded.networks[0];
    assert_eq!(net.label.as_deref(), Some("Main"));
    let keys: Vec<&str> = net.properties.keys().map(String::as_str).collect();
    assert_eq!(keys, ["domain", "priority2"], "label/protocol excluded, rest sorted");
}

/// A network with no extra keys returns an empty map (existing files load unchanged).
#[test]
fn network_without_extras_has_empty_properties() {
    let loaded = load_from_patch("network N {\n  protocol: \"Dante\"\n}\n", "").expect("load");
    assert!(loaded.networks[0].properties.is_empty());
}

/// Docs claim PatchLang keywords outside the property-key list are dropped by the parser.
/// `slot` is one of them; a following ordinary key must still load.
#[test]
fn keyword_keys_outside_property_list_do_not_become_properties() {
    let src = "template T { meta { model: \"T\" } }\ninstance D is T {\n  in: \"x\"\n  clock_source: \"internal\"\n}\n";
    let loaded = load_from_patch(src, "");
    match loaded {
        Ok(out) => assert!(
            !out.instances[0].properties.contains_key("in"),
            "keyword key must not survive as a property: {:?}",
            out.instances[0].properties
        ),
        Err(e) => panic!("a keyword key should be dropped, not make load fail: {e}"),
    }
}
