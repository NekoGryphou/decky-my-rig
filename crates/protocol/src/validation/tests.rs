use super::valid_name;

#[test]
fn names_are_trimmed_bounded_visible_text() {
    assert!(valid_name("Living room device"));
    assert!(valid_name("Salon – Steam Deck 🎮"));
    assert!(valid_name(&"é".repeat(80)));
    for name in [
        "",
        " Device",
        "Device\n",
        &"x".repeat(81),
        // - Invisible or reordering characters can make one name look like another.
        "Device\u{200B}",
        "Device\u{202E}kceD",
        "\u{2066}Device\u{2069}",
        "\u{FEFF}Device",
        "Device\u{00AD}",
    ] {
        assert!(!valid_name(name), "{name:?}");
    }
}

#[test]
fn the_shared_name_fixture_holds_on_this_side() {
    // ! The Windows app reads the same file, so both sides have to agree on every name.
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/local-control/names.json"
    ))
    .unwrap();
    for name in fixture["valid"].as_array().unwrap() {
        assert!(valid_name(name.as_str().unwrap()), "{name}");
    }
    for case in fixture["invalid"].as_array().unwrap() {
        assert!(
            !valid_name(case["name"].as_str().unwrap()),
            "{}",
            case["why"]
        );
    }
}
