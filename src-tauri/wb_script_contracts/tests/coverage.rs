use std::collections::HashSet;
use std::fs;
use wb_script_contracts::{load_scripts, parse_groups_wb_keys};

#[test]
fn scripts_json_matches_groups_rs() {
    let groups_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../src/scripts/groups.rs");
    let groups = fs::read_to_string(&groups_path).expect("groups.rs");
    let from_groups: HashSet<_> = parse_groups_wb_keys(&groups).into_iter().collect();
    let from_catalog: HashSet<_> = load_scripts()
        .expect("scripts.json")
        .into_iter()
        .map(|s| s.key)
        .collect();
    let missing: Vec<_> = from_groups.difference(&from_catalog).cloned().collect();
    let extra: Vec<_> = from_catalog.difference(&from_groups).cloned().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "coverage mismatch missing={missing:?} extra={extra:?}"
    );
}
