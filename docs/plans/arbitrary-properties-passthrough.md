# Plan — Arbitrary property passthrough (instance / network / ring)

**Status:** Reid answered 2026-11-02 (section 3). Ready for Phase 1; Phase 2 and network emit not needed. Nothing implemented yet.
**Requested by:** Reid (Frontend), for the audio word-clock feature.
**Repos:** `SignalCanvasLang` (Rust + WASM). Frontend work is Reid's.

---

## 1. The request

Let arbitrary `key: "value"` properties survive WASM load and emit, so they survive save and reload:

- **Instance blocks:** extra keys on an instance. Example:
  `instance FOH_Engine is dLIVE_DM0 { clock: "source=Slot_1_Dante;rate=96k"  clock_p_Slot_1_Dante: "rate=96k;role=follower;src=on" }`
- **Network and ring blocks:** the same. Example: `network PTPv2_Domain_0 { protocol: "PTPv2"  domain: "0" }`
- **Emit side:** accept the same properties in the emit input and write them out.

Why: per-port clock settings (rate, SRC, leader/follower, AES67 mode, PTPv2 priority) and the PTPv2 domain number have nowhere to live. `member` lines can't carry properties, and Reid doesn't want device config in the sidecar. It would also unblock the `outputCardAllocations` and `ioBlockExtractions` TODOs in `SignalCanvasFrontend/src/lang/emitterSidecar.ts`.

## 2. What we found

- **The pattern exists.** `ChannelLabelOutput.properties` and `ConnectionLoadOutput.properties` are `BTreeMap<String,String>` passthroughs. Emit-side inputs take the same field with `#[serde(default)]` so older payloads still deserialize (`canvas_input.rs`, `canvas_emit/mod.rs:301`).
- **Parser needs no change.** The AST already keeps every property on `InstanceDecl`, `NetworkDecl` and `RingDecl`. Keys like `clock`, `domain` and `clock_p_*` lex as plain identifiers.
- **Where properties are dropped today:**
  - `canvas_load.rs:234` builds the instance property map, then reads only `location`, `rf_band`, `rf_active_channels` and `iem_modes`.
  - `canvas_load.rs:84` reads only `protocol` and `label` from a network. Rings read only `protocol`.
- **No network emit in Lang.** `CanvasEmitInput` has no networks and the builder has no `add_network`. The frontend writes network blocks as raw strings in `emitterPipeline.ts` (`formatNetworkBlocks`).
- **Ring containers have no `instance` block.** Emit skips them as instances and writes only a `ring` block (`canvas_emit/mod.rs:187`). Instance-level properties do nothing for them.
- **DRC already reads two instance properties:** `aes67_mode` and `multicast_prefix` (`drc/convention.rs:213`, `drc/flow.rs:336`).
- **Keyword keys are silently dropped.** `is_property_key()` (`parser.rs:121`) accepts identifiers plus `label`, `stream`, `route`, `bus`, `routing`, `config`. A key that is any other keyword (`in`, `out`, `io`, `is`, `use`, `slot`, `meta`, `ports`, `flag`, `signal`, `for`, `over`, `generate`) is skipped with no error. Not a problem for Reid's planned keys.
- **Correction to Reid's message:** `optocore_id` is not in `InstanceLoadOutput`. It lives in the sidecar. This feature would let it move into the `.patch`.
- `canvas_load.rs` is already 929 lines, over the 700 limit in `ClaudeCodeRules.md`. New code goes in a new file.

## 3. Questions sent to Reid

Answers go here when we have them.

1. **Opaque or understood?** Lang stores `clock: "..."` and never looks inside, so no checks like "different sample rates on a connection" or "two leaders on one domain". Does he want clock checking in Lang eventually, or does the UI own it? If checking is wanted, a free-form string is the wrong format and we design it properly now.
   - Answer: (Reid, 2026-11-02) UI owns all clock checks; Lang treats values as opaque. Packed string rejected. Reid will use one readable key per setting with plain string values, e.g. `clock_source: "internal"`, `clock_rate: "96k"`, `clock_p_Slot_1_Dante_rate: "96k"`, `clock_p_Slot_1_Dante_role: "follower"`, `clock_p_Slot_1_Dante_src: "on"`. Open: he asks if we prefer different key names.
2. **One-off or general mechanism?** Is this only for clocking, or the standard way any per-device setting is saved (output cards, I/O blocks, etc.)? If general, we document it as a language feature.
   - Answer: (Reid, 2026-11-02) General mechanism, yes. Document as a real language feature. Also usable for output-card / I/O-block TODOs. Sorted keys, string-only values, empty by default. Correction: network blocks are written as text by FrontendV1 (`formatNetworkBlocks`), so NO network emit is needed from Lang; networks only need load to return properties (just the PTPv2 domain). `InstanceLoadOutput` has no `optocore_id` field, ignore that line.
3. **Do clock-bearing devices include ring-based gear (OptoCore etc.)?** If yes, ring properties are required and need their own emit field. If no, skip rings.
   - Answer: (Reid, 2026-11-02) Rings need exactly one leader, stored per port on the member instance, so the instance map covers it. Ring-level map is optional / nice to have later. => Phase 2 is NOT required now.
4. **Who else reads these settings?** Hand-editors of the `.patch`, the Source tab, MCP/LLM workflows, exports? If anything beyond the canvas reads them, agree a readable convention (probably better than packing `rate=96k;role=follower` into one string).
   - Answer: (Reid, 2026-11-02) Source tab and hand-editors of `.patch` for sure; MCP/LLM builder and exports likely. Wants everything LLM/MCP friendly. => readable key-per-setting convention (see Q1).
5. **When does he need it?**
   - Answer: (Reid, 2026-11-02) Timing not stated in his reply; his original ask was a rough size + landing date. Everything else in his feature can be built meanwhile.

## 4. Defaults we're assuming (change only if Reid objects)

- Values are strings only. `domain: 0` and `domain: "0"` both load as `"0"` and re-emit quoted.
- Properties come back sorted by key, not authored order (keeps emit → load → emit stable; same as connection and label properties).
- Network blocks keep being written by his `formatNetworkBlocks`. Lang only reads network properties on load. (Alternative: add `NetworkEmitInput` + `add_network`, about half a day more.)
- Lang never validates keys or values. In particular it does not check that a port named inside `clock_p_<Port>` exists. If the key is built from a label that `sanitize_id` handles differently, or the port is renamed, the key is silently orphaned. Reid owns that.
- `aes67_mode` and `multicast_prefix` already trigger DRC rules. Worth telling him in case his AES67 setting reuses those names.

## 5. Implementation plan

### Phase 1: instance and network load/emit

Load (`crates/patchlang/src/builder/`):
1. `canvas_output.rs`: add `properties: BTreeMap<String,String>` to `InstanceLoadOutput` and `NetworkLoadOutput`.
2. New file (e.g. `canvas_properties.rs`): helper returning leftover properties, i.e. keys with no dedicated field.
   - Instance dedicated keys: `location`, `rf_band`, `rf_active_channels`, `iem_modes`.
   - Network dedicated keys: `protocol`, `label`.
   - A key lives in exactly one place. `leftover_label_props` already follows this rule.
3. `canvas_load.rs`: call the helper at the instance construction (~line 466) and the network construction (~line 84).

Emit:
4. `canvas_input.rs`: add `#[serde(default)] properties: BTreeMap<String,String>` to `InstanceEmitInput`.
5. `canvas_emit/mod.rs` `build_instance_decl` (~line 404): after the dedicated keys, append each extra property with `kv_str`. Skip keys that collide with a dedicated one.

### Phase 2: rings (only if Q3 is yes)
- Add `properties` to `RingLoadOutput` and load it from `Statement::Ring`.
- Add a ring properties field to `InstanceEmitInput` and write it into the `RingDecl` in emit Phase 5.

### Phase 3: tests, docs, release
- Tests: extend `builder_tests/canvas_roundtrip_tests.rs` and `property_tests.rs` (commit `ffc2ed3`). Cover:
  - Extra properties on an instance and a network survive load → emit → load.
  - Empty or absent map leaves existing output byte-identical.
  - Dedicated keys don't duplicate.
  - Old payloads without `properties` still deserialize.
  - Ordering is stable; special characters escaped.
  - Reid's examples (`clock_p_Slot_1_Dante`, `domain`).
- Regenerate ts-rs bindings in `crates/patchlang/bindings/`.
- Docs: note in `docs/language-reference.md` and `SPEC.md` that instance, ring and network blocks accept arbitrary string properties; list reserved keys and the keyword-as-key caveat.
- Release: bump from 0.3.4, rebuild WASM, then copy into all three places or the compiler breaks on a glue/binary mismatch:
  - backend's vendored `patchlang-wasm`
  - admin-ui `vendor/` (glue)
  - admin-ui `public/` (binary)

### Not changing
Lexer, parser, grammar, DRC rules.

## 6. Estimate
- Instance + network-load: about half a day, plus WASM rebuild and the three-way sync.
- Network emit in Lang (optional): about half a day more.
- Rings: small, same pattern.

## 7. How this changes depending on Reid's answers
- **Q1 says Lang should understand clock settings:** stop. Don't ship free-form strings. Design a real grammar (and DRC rules) first.
- **Q3 says yes:** do Phase 2.
- **Q2 says general mechanism:** write the docs in Phase 3 as a proper language feature, not a footnote.
- **Q4 says others read it:** agree the string convention before shipping, since it's hard to change once files exist.
- **Everything canvas-only and opaque:** proceed with Phase 1 as written.
