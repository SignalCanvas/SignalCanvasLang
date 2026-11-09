//! Canvas emit tests: templates, ports, and channel labels.

use super::canvas_test_helpers::*;
use crate::builder::canvas_emit::emit_from_canvas_input;
use crate::builder::canvas_input::*;
use crate::builder::canvas_load::load_from_patch;
use std::collections::{BTreeMap, HashMap};

#[test]
fn canvas_emit_input_deserializes_empty() {
    let json = r#"{"instances":[],"connections":[],"manufacturer_cards":[]}"#;
    let input: CanvasEmitInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.instances.len(), 0);
    assert_eq!(input.connections.len(), 0);
}

#[test]
fn canvas_emit_input_deserializes_instance_with_interfaces() {
    let json = r#"{
        "instances": [{
            "name": "FOH_Console",
            "device_type": "device",
            "manufacturer": "Yamaha",
            "model": "CL5",
            "category": "Console",
            "kind": null,
            "location": null,
            "dante_chipset": null,
            "rf_subtype": null,
            "rf_min_channels": null,
            "rf_max_channels": null,
            "rf_band": null,
            "interfaces": [{
                "id": "dante_pri",
                "label": "Dante_Pri",
                "direction": "io",
                "connector": "etherCON",
                "transport": "Dante",
                "channel_count": 32,
                "attributes": ["primary"]
            }],
            "card_slot_groups": [],
            "installed_cards": [],
            "channel_labels": {},
            "route_rules": [],
            "instance_routes": [],
            "internal_buses": [],
            "tx_streams": [],
            "rx_streams": [],
            "is_ring_container": false,
            "ring_protocol": null,
            "ring_members": [],
            "rf_active_channels": null,
            "iem_modes": null
        }],
        "connections": [],
        "manufacturer_cards": []
    }"#;
    let input: CanvasEmitInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.instances.len(), 1);
    assert_eq!(input.instances[0].name, "FOH_Console");
    assert_eq!(input.instances[0].interfaces.len(), 1);
    assert_eq!(
        input.instances[0].interfaces[0].transport.as_deref(),
        Some("Dante")
    );
}

#[test]
fn emit_produces_template_and_instance() {
    let input = CanvasEmitInput {
        instances: vec![make_simple_instance(
            "FOH_Console",
            "CL5",
            "Yamaha",
            vec![make_interface(
                "d1",
                "Dante_Pri",
                "io",
                Some("Dante"),
                32,
                vec!["primary"],
            )],
        )],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch = emit_from_canvas_input(input).unwrap();
    assert!(
        patch.contains("template CL5"),
        "should emit template:\n{patch}"
    );
    assert!(
        patch.contains("instance FOH_Console is CL5"),
        "should emit instance:\n{patch}"
    );
}

#[test]
fn emit_splits_dante_io_into_in_and_out() {
    let input = CanvasEmitInput {
        instances: vec![make_simple_instance(
            "Stage_Left",
            "Rio3224",
            "Yamaha",
            vec![make_interface(
                "d1",
                "Dante_Pri",
                "io",
                Some("Dante"),
                32,
                vec!["primary"],
            )],
        )],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch = emit_from_canvas_input(input).unwrap();
    assert!(
        patch.contains("Dante_Pri_In[1..32]: in"),
        "Dante io must split to _In:\n{patch}"
    );
    assert!(
        patch.contains("Dante_Pri_Out[1..32]: out"),
        "Dante io must split to _Out:\n{patch}"
    );
    assert!(
        !patch.contains("Dante_Pri: io"),
        "must NOT emit unsplit io port:\n{patch}"
    );
}

#[test]
fn emit_optocore_stays_as_io() {
    let input = CanvasEmitInput {
        instances: vec![make_simple_instance(
            "CL5_1",
            "CL5",
            "Yamaha",
            vec![make_interface(
                "opt1",
                "OptoCore_A",
                "io",
                Some("OptoCore"),
                0,
                vec![],
            )],
        )],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch = emit_from_canvas_input(input).unwrap();
    assert!(
        patch.contains("OptoCore_A: io"),
        "OptoCore must stay as io:\n{patch}"
    );
    assert!(
        !patch.contains("OptoCore_A_In") && !patch.contains("OptoCore_A_Out"),
        "OptoCore must not split:\n{patch}"
    );
}

#[test]
fn emit_channel_labels_appear_in_config_block() {
    let mut labels = HashMap::new();
    labels.insert(
        "d1".to_string(),
        vec![
            ChannelLabelEmitInput {
                channel_index: 1,
                label: "Lead Vocal".into(),
                phantom: true,
                propagated: false,
                source_type: None,
                capsule: None,
                rf_band: None,
                ..Default::default()
            },
            ChannelLabelEmitInput {
                channel_index: 2,
                label: "Kick Drum".into(),
                phantom: false,
                propagated: false,
                source_type: None,
                capsule: None,
                rf_band: None,
                ..Default::default()
            },
        ],
    );
    let mut inst = make_simple_instance(
        "FOH_Console",
        "CL5",
        "Yamaha",
        vec![make_interface(
            "d1",
            "Dante_Pri",
            "io",
            Some("Dante"),
            32,
            vec![],
        )],
    );
    inst.channel_labels = labels;
    let input = CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch = emit_from_canvas_input(input).unwrap();
    assert!(
        patch.contains("config FOH_Console"),
        "should emit config block:\n{patch}"
    );
    assert!(
        patch.contains("Lead Vocal"),
        "label text must appear:\n{patch}"
    );
    assert!(
        patch.contains("phantom"),
        "phantom flag must appear:\n{patch}"
    );
}

#[test]
fn emit_channel_label_on_card_slot_port() {
    // Riedel Artist 64 with AES67-108 G2 card in slot 1.
    // Label on card's AES67[1] = "Main Mix L".
    // Bug: emit only searches chassis interfaces, falls through to sanitize_id,
    // then builder.set_label() throws PortNotFound and aborts the entire emit.
    let card = CardEmitInput {
        template_name: "AES67_108_G2".into(),
        manufacturer: Some("Riedel".into()),
        model: "AES67-108 G2".into(),
        fits: "Artist_Slot".into(),
        interfaces: vec![make_interface(
            "card_aes67",
            "AES67",
            "io",
            Some("AES67"),
            64,
            vec![],
        )],
    };
    let mut inst = make_simple_instance(
        "Artist_64",
        "Artist64",
        "Riedel",
        vec![make_interface(
            "mgmt",
            "Mgmt",
            "io",
            Some("Ethernet_Mgmt"),
            0,
            vec![],
        )],
    );
    inst.installed_cards = vec![InstalledCardEmitInput {
        slot_label: "Card_Slot".into(),
        slot_index: 1,
        card_template_name: "AES67_108_G2".into(),
    }];
    let mut labels = HashMap::new();
    labels.insert(
        "card_aes67".into(),
        vec![ChannelLabelEmitInput {
            channel_index: 1,
            label: "Main Mix L".into(),
            phantom: false,
            propagated: false,
            source_type: None,
            capsule: None,
            rf_band: None,
            ..Default::default()
        }],
    );
    inst.channel_labels = labels;
    let input = CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![card],
    };
    let patch = emit_from_canvas_input(input).unwrap();
    assert!(
        patch.contains("config Artist_64"),
        "config block must be emitted:\n{patch}"
    );
    assert!(
        patch.contains("Main Mix L"),
        "label text must appear:\n{patch}"
    );
    assert!(
        patch.contains("label AES67_In[1]: \"Main Mix L\""),
        "label must use correct directional port name from card interface:\n{patch}"
    );
    assert!(
        !patch.contains("card_aes67"),
        "label must NOT emit raw interface id as port name:\n{patch}"
    );
}


// ---------------------------------------------------------------------------
// Arbitrary instance/network property passthrough (word-clock feature)
// ---------------------------------------------------------------------------

/// Build a minimal instance carrying the given extra properties.
fn instance_with_props(props: &[(&str, &str)]) -> InstanceEmitInput {
    let mut inst = make_simple_instance("FOH_Engine", "dLIVE_DM0", "AllenHeath", vec![]);
    inst.properties = props
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    inst
}

/// Reid's instance example: extra keys survive emit -> load with values intact.
#[test]
fn instance_extra_properties_survive_emit_load() {
    // Reid's word-clock keys, deliberately declared out of sorted order.
    let inst = instance_with_props(&[
        ("clock_source", "internal"),
        ("clock_rate", "96k"),
        ("clock_p_Slot_1_Dante_rate", "96k"),
        ("clock_p_Slot_1_Dante_role", "follower"),
        ("clock_p_Slot_1_Dante_src", "on"),
    ]);
    let input = CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch = emit_checked(input, "instance extra props");
    let loaded = load_from_patch(&patch, "").expect("load");
    let out = &loaded.instances[0];
    assert_eq!(out.properties.get("clock_source").map(String::as_str), Some("internal"));
    assert_eq!(
        out.properties.get("clock_p_Slot_1_Dante_role").map(String::as_str),
        Some("follower")
    );
    assert_eq!(out.properties.len(), 5, "only the 5 extra keys, no more: {:?}", out.properties);
}

/// Full emit -> load -> emit -> load: the property map is byte-stable across the loop.
#[test]
fn instance_extra_properties_round_trip_is_stable() {
    let make = || CanvasEmitInput {
        instances: vec![instance_with_props(&[
            ("clock_source", "internal"),
            ("clock_p_Slot_1_Dante_src", "on"),
        ])],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let patch1 = emit_from_canvas_input(make()).unwrap();
    let loaded1 = load_from_patch(&patch1, "").unwrap();
    // Rebuild an emit input from the load output, threading properties back.
    let mut inst2 = make_simple_instance("FOH_Engine", "dLIVE_DM0", "AllenHeath", vec![]);
    inst2.properties = loaded1.instances[0].properties.clone();
    let patch2 = emit_from_canvas_input(CanvasEmitInput {
        instances: vec![inst2],
        connections: vec![],
        manufacturer_cards: vec![],
    })
    .unwrap();
    assert_eq!(patch1, patch2, "emit -> load -> emit must be byte-identical");
}

/// An empty properties map emits nothing extra: output identical to no-properties.
#[test]
fn empty_properties_map_is_byte_identical() {
    let with_empty = instance_with_props(&[]);
    let mut no_props = make_simple_instance("FOH_Engine", "dLIVE_DM0", "AllenHeath", vec![]);
    no_props.properties = BTreeMap::new();
    let wrap = |inst| CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![],
    };
    let a = emit_from_canvas_input(wrap(with_empty)).unwrap();
    let b = emit_from_canvas_input(wrap(no_props)).unwrap();
    assert_eq!(a, b, "empty map must not change emitted output");
    assert!(!a.contains("clock"), "no stray keys:\n{a}");
}

/// A property whose key collides with a dedicated field is skipped on emit (the
/// dedicated field wins) and never written twice.
#[test]
fn dedicated_key_collision_is_not_duplicated() {
    let mut inst = make_simple_instance("FOH_Engine", "dLIVE_DM0", "AllenHeath", vec![]);
    inst.location = Some("Rack A".into());
    inst.properties.insert("location".into(), "SHOULD_BE_SKIPPED".into());
    inst.properties.insert("clock_source".into(), "internal".into());
    let patch = emit_from_canvas_input(CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![],
    })
    .unwrap();
    assert!(
        patch.contains("location: \"Rack A\""),
        "dedicated field value wins:\n{patch}"
    );
    assert!(
        !patch.contains("SHOULD_BE_SKIPPED"),
        "colliding property must be skipped:\n{patch}"
    );
    assert!(
        patch.contains("clock_source: \"internal\""),
        "non-colliding property still emitted:\n{patch}"
    );
    // Exactly one occurrence of the location key.
    assert_eq!(patch.matches("location:").count(), 1, "location written once:\n{patch}");
}

/// Properties emit sorted by key regardless of insertion order (BTreeMap), so the
/// output is deterministic.
#[test]
fn properties_emit_in_sorted_order() {
    let inst = instance_with_props(&[
        ("zebra", "z"),
        ("alpha", "a"),
        ("mike", "m"),
    ]);
    let patch = emit_from_canvas_input(CanvasEmitInput {
        instances: vec![inst],
        connections: vec![],
        manufacturer_cards: vec![],
    })
    .unwrap();
    let ia = patch.find("alpha:").expect("alpha emitted");
    let im = patch.find("mike:").expect("mike emitted");
    let iz = patch.find("zebra:").expect("zebra emitted");
    assert!(ia < im && im < iz, "keys must be sorted: alpha<mike<zebra\n{patch}");
}

/// A value with a quote or newline round-trips escaped and back (#35/D026 path).
#[test]
fn property_value_with_special_chars_round_trips() {
    let inst = instance_with_props(&[("note", "The \"Big\" Mix")]);
    let patch = emit_checked(
        CanvasEmitInput {
            instances: vec![inst],
            connections: vec![],
            manufacturer_cards: vec![],
        },
        "special-char property",
    );
    let loaded = load_from_patch(&patch, "").unwrap();
    assert_eq!(
        loaded.instances[0].properties.get("note").map(String::as_str),
        Some("The \"Big\" Mix"),
        "quote must survive the round trip:\n{patch}"
    );
}

/// Old payload JSON with no `properties` key still deserializes (serde default).
#[test]
fn old_payload_without_properties_deserializes() {
    let json = r#"{
        "instances": [{
            "name": "FOH_Engine", "device_type": "device", "manufacturer": null,
            "model": "dLIVE_DM0", "category": null, "kind": null, "location": null,
            "dante_chipset": null, "rf_subtype": null, "rf_min_channels": null,
            "rf_max_channels": null, "rf_band": null, "rf_active_channels": null,
            "iem_modes": null, "interfaces": [], "card_slot_groups": [],
            "installed_cards": [], "channel_labels": {}, "route_rules": [],
            "instance_routes": [], "internal_buses": [], "tx_streams": [],
            "rx_streams": [], "is_ring_container": false, "ring_protocol": null,
            "ring_members": []
        }],
        "connections": [], "manufacturer_cards": []
    }"#;
    let input: CanvasEmitInput = serde_json::from_str(json).expect("old payload must deserialize");
    assert!(input.instances[0].properties.is_empty());
}

/// Reid's network example: `domain` survives load (networks are load-only; the
/// frontend writes network blocks as text). `domain: 0` and `domain: "0"` both load
/// as the string "0".
#[test]
fn network_extra_property_survives_load() {
    let src = r#"
network PTPv2_Domain_0 {
  protocol: "PTPv2"
  domain: "0"
}
"#;
    let loaded = load_from_patch(src, "").expect("network load");
    let net = &loaded.networks[0];
    assert_eq!(net.protocol.as_deref(), Some("PTPv2"));
    assert_eq!(net.properties.get("domain").map(String::as_str), Some("0"));
    // protocol is dedicated — must NOT also appear in the leftover map.
    assert!(!net.properties.contains_key("protocol"), "dedicated key leaked into map");
    assert_eq!(net.properties.len(), 1, "only domain is leftover: {:?}", net.properties);
}

/// Unquoted numeric network property loads as its string form ("0"), per the
/// string-only-values default.
#[test]
fn network_numeric_property_loads_as_string() {
    let src = "network N {\n  protocol: \"PTPv2\"\n  domain: 0\n}\n";
    let loaded = load_from_patch(src, "").expect("load");
    assert_eq!(
        loaded.networks[0].properties.get("domain").map(String::as_str),
        Some("0"),
        "domain: 0 must load as the string \"0\""
    );
}
