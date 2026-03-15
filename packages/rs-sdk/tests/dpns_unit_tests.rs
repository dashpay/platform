use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
};

#[test]
fn test_dpns_validation_functions() {
    // Test username validation
    let test_names = vec![
        ("alice", true),
        ("test", true),
        ("dash", true),
        ("a", false),
        ("ab", false),
        ("123", true),
        ("test-name", true),
        ("test--name", false),
        ("-test", false),
        ("test-", false),
    ];

    for (name, expected_valid) in test_names {
        assert_eq!(
            is_valid_username(name),
            expected_valid,
            "is_valid_username({name}) should be {expected_valid}"
        );
    }

    // Test homograph conversion
    let test_conversions = vec![
        ("alice", "a11ce"),
        ("bob", "b0b"),
        ("COOL", "c001"),
        ("test123", "test123"),
        ("ali", "a11"),
        ("dash", "dash"),
    ];

    for (input, expected) in test_conversions {
        let result = convert_to_homograph_safe_chars(input);
        assert_eq!(
            result, expected,
            "convert_to_homograph_safe_chars({input}) should be {expected}"
        );
    }

    // Test contested username check
    let test_contested = vec![
        ("abc", true),                   // 3 chars
        ("test", true),                  // 4 chars
        ("alice", true),                 // 5 chars, only lowercase
        ("Alice", true),                 // Converts to "a11ce" which is contested
        ("test-name", true),             // Hyphens are allowed in contested names
        ("test123", false),              // Has numbers
        ("a", false),                    // Too short
        ("ab", false),                   // Too short
        ("twentycharacterslong", false), // 20 chars, too long for contested
    ];

    for (name, expected) in test_contested {
        assert_eq!(
            is_contested_username(name),
            expected,
            "is_contested_username({}) should be {}",
            name,
            expected
        );
    }
}

#[test]
fn test_dpns_edge_cases() {
    // Test minimum and maximum length usernames
    let min_name = "abc";
    let max_name = "a".repeat(63);
    let too_long = "a".repeat(64);

    assert!(is_valid_username(min_name));
    assert!(is_valid_username(&max_name));
    assert!(!is_valid_username(&too_long));

    // Test special characters
    let special_tests = vec![
        "test_name",  // underscore
        "test.name",  // dot
        "test@name",  // at
        "test name",  // space
        "test/name",  // slash
        "test\\name", // backslash
        "test:name",  // colon
        "test;name",  // semicolon
        "test'name",  // apostrophe
        "test\"name", // quote
    ];

    for name in special_tests {
        assert!(
            !is_valid_username(name),
            "special-character username should be invalid: {name}"
        );
    }

    // Test Unicode/international characters
    let unicode_tests = vec![
        "café",     // French
        "münchen",  // German
        "北京",     // Chinese
        "🚀rocket", // Emoji
        "user₿",    // Bitcoin symbol
    ];

    for name in unicode_tests {
        assert!(
            !is_valid_username(name),
            "unicode username should be invalid: {name}"
        );
    }
}

#[test]
fn test_dpns_homograph_safety() {
    // Test various homograph attacks
    let homograph_tests = vec![
        ("paypal", "paypa1"),       // lowercase L to 1
        ("google", "g00g1e"),       // o to 0, l to 1
        ("microsoft", "m1cr0s0ft"), // i to 1, o to 0
        ("admin", "adm1n"),         // i to 1
        ("root", "r00t"),           // o to 0
        ("alice", "a11ce"),         // l to 1, i to 1
        ("bill", "b111"),           // i to 1, l to 1
        ("cool", "c001"),           // o to 0, l to 1
        ("lol", "101"),             // l to 1, o to 0
        ("oil", "011"),             // o to 0, i to 1, l to 1
    ];

    for (input, expected) in homograph_tests {
        let result = convert_to_homograph_safe_chars(input);
        assert_eq!(
            result, expected,
            "convert_to_homograph_safe_chars({input}) should be {expected}"
        );
    }

    // Test that the conversion is idempotent
    let test_names = vec!["alice", "bob", "cool", "test"];

    for name in test_names {
        let once = convert_to_homograph_safe_chars(name);
        let twice = convert_to_homograph_safe_chars(&once);
        assert_eq!(once, twice, "conversion should be idempotent for '{name}'");
    }
}
