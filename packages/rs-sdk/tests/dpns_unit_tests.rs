use dash_sdk::platform::dpns_usernames::{convert_to_homograph_safe_chars, is_valid_username, is_contested_username};

#[test]
fn test_dpns_validation_functions() {
    println!("Testing DPNS validation functions with values from docs...\n");

    // Test username validation
    println!("1. Testing is_valid_username:");
    let test_names = vec!["alice", "test", "dash", "a", "ab", "123", "test-name", "test--name", "-test", "test-"];
    
    for name in test_names {
        let is_valid = is_valid_username(name);
        println!("   '{}' is {}", name, if is_valid { "✅ VALID" } else { "❌ INVALID" });
    }
    println!();

    // Test homograph conversion
    println!("2. Testing convert_to_homograph_safe_chars:");
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
        let matches = result == expected;
        println!("   '{}' → '{}' {}", input, result, if matches { "✅" } else { "❌ (expected: {})" });
        if !matches {
            println!("      Expected: {}", expected);
        }
    }
    println!();

    // Test contested username check
    println!("3. Testing is_contested_username:");
    let test_contested = vec![
        ("abc", true),        // 3 chars
        ("test", true),       // 4 chars
        ("alice", true),      // 5 chars, only lowercase
        ("Alice", true),      // Converts to "a11ce" which is contested
        ("test-name", true),  // Hyphens are allowed in contested names
        ("test123", false),   // Has numbers
        ("a", false),         // Too short
        ("ab", false),        // Too short
        ("twentycharacterslong", false), // 20 chars, too long for contested
    ];
    
    for (name, expected) in test_contested {
        let result = is_contested_username(name);
        let matches = result == expected;
        println!("   '{}' is {} contested {}", 
            name, 
            if result { "🔥" } else { "📝" },
            if matches { "✅" } else { "❌" }
        );
    }
}

#[test]
fn test_dpns_edge_cases() {
    println!("\nTesting DPNS edge cases...\n");

    // Test minimum and maximum length usernames
    let min_name = "abc";
    let max_name = "a".repeat(63);
    let too_long = "a".repeat(64);
    
    println!("Length tests:");
    println!("   3 chars '{}': {}", min_name, if is_valid_username(min_name) { "✅ VALID" } else { "❌ INVALID" });
    println!("   63 chars: {}", if is_valid_username(&max_name) { "✅ VALID" } else { "❌ INVALID" });
    println!("   64 chars: {}", if is_valid_username(&too_long) { "✅ VALID (should be invalid!)" } else { "❌ INVALID (correct)" });
    
    // Test special characters
    println!("\nSpecial character tests:");
    let special_tests = vec![
        "test_name",    // underscore
        "test.name",    // dot
        "test@name",    // at
        "test name",    // space
        "test/name",    // slash
        "test\\name",   // backslash
        "test:name",    // colon
        "test;name",    // semicolon
        "test'name",    // apostrophe
        "test\"name",   // quote
    ];
    
    for name in special_tests {
        println!("   '{}': {}", name, if is_valid_username(name) { "✅ VALID" } else { "❌ INVALID" });
    }
    
    // Test Unicode/international characters
    println!("\nUnicode character tests:");
    let unicode_tests = vec![
        "café",         // French
        "münchen",      // German
        "北京",         // Chinese
        "🚀rocket",     // Emoji
        "user₿",        // Bitcoin symbol
    ];
    
    for name in unicode_tests {
        println!("   '{}': {}", name, if is_valid_username(name) { "✅ VALID" } else { "❌ INVALID" });
    }
}

#[test] 
fn test_dpns_homograph_safety() {
    println!("\nTesting DPNS homograph safety conversions...\n");
    
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
        println!("   '{}' → '{}' (expected: {})", input, result, expected);
    }
    
    // Test that the conversion is idempotent
    println!("\nIdempotency test (converting twice should give same result):");
    let test_names = vec!["alice", "bob", "cool", "test"];
    
    for name in test_names {
        let once = convert_to_homograph_safe_chars(name);
        let twice = convert_to_homograph_safe_chars(&once);
        let matches = once == twice;
        println!("   '{}' → '{}' → '{}' {}", 
            name, once, twice, 
            if matches { "✅ Idempotent" } else { "❌ Not idempotent!" }
        );
    }
}