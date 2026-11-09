use std::collections::HashSet;
use wb_script_contracts::{
    assert_token_bucket_capacity_one, load_rates, load_scripts, rates_equal,
};

#[test]
fn ours_matches_expected_and_capacity_one() {
    let scripts = load_scripts().expect("scripts.json");
    let ours = load_rates("rates_ours.json").expect("rates_ours");
    let expected = load_rates("rates_expected.json").expect("rates_expected");
    assert_token_bucket_capacity_one(&ours).unwrap();
    assert_token_bucket_capacity_one(&expected).unwrap();

    let api_keys: HashSet<_> = scripts
        .iter()
        .filter(|s| s.kind == "api")
        .map(|s| s.key.as_str())
        .collect();

    for key in &api_keys {
        let o = ours
            .get(*key)
            .unwrap_or_else(|| panic!("rates_ours missing {key}"));
        let e = expected
            .get(*key)
            .unwrap_or_else(|| panic!("rates_expected missing {key}"));
        assert!(
            rates_equal(o, e),
            "rate mismatch for {key}: ours={o:?} expected={e:?}"
        );
    }

    for k in ours.keys() {
        assert!(
            api_keys.contains(k.as_str()),
            "rates_ours has unknown key {k}"
        );
    }
}
