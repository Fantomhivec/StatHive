use serde_json::Value;
use std::fs;
use wb_script_contracts::{fixtures_dir, load_fields, load_scripts, path_exists_typed};

#[test]
fn ok_fixtures_match_required_fields() {
    let scripts = load_scripts().expect("scripts.json");
    for s in scripts.iter().filter(|s| s.kind == "api") {
        let fields = load_fields(&s.key).unwrap_or_else(|e| panic!("{}: {e}", s.key));
        let ok_path = fixtures_dir().join(&s.key).join("ok.json");
        let raw = fs::read_to_string(&ok_path).unwrap_or_else(|e| panic!("{}: {e}", ok_path.display()));
        let v: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{}: {e}", s.key));
        for req in &fields.required {
            path_exists_typed(&v, &req.path, &req.ty)
                .unwrap_or_else(|e| panic!("{} ok.json: {e}", s.key));
        }
        // empty + error must parse
        for name in ["empty.json", "error.json"] {
            let p = fixtures_dir().join(&s.key).join(name);
            let raw = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let _: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{} {name}: {e}", s.key));
        }
    }
}
