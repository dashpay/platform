use std::fs;
use std::process::Command;
use toml::Value;

fn main() {
    let crates = [
        "rs-sdk",
        "rs-drive-abci",
        "rs-dpp",
        "rs-drive",
        "rs-drive-proof-verifier",
    ];

    for specific_crate in crates {
        check_crate(specific_crate)
    }
}

fn check_crate(crate_name: &str) {
    // Construct the path to the Cargo.toml file for each crate
    let cargo_toml_path = format!("packages/{}/Cargo.toml", crate_name);

    // Read and parse the Cargo.toml file
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|_| panic!("Failed to read Cargo.toml for {}", crate_name));

    let cargo_toml: Value = cargo_toml_content
        .parse()
        .expect("Failed to parse Cargo.toml");

    let features = cargo_toml
        .get("features")
        .expect("No features in Cargo.toml");

    let name = cargo_toml
        .get("package")
        .expect("No package in Cargo.toml")
        .get("name")
        .expect("expected name in Cargo.toml");

    println!("Checking crate {} with default features", crate_name);

    // Change directory to the crate's directory and run cargo check for the specific feature
    let status = Command::new("cargo")
        .current_dir(format!("packages/{}", crate_name)) // Set the current directory to the crate's directory
        .arg("check")
        .status()
        .expect("Failed to execute cargo check");

    if !status.success() {
        println!(
            "Feature check failed in crate {} with default features",
            crate_name
        );
        println!("cargo check -p {}", name);
        std::process::exit(1);
    }

    println!("Checking crate {} with no default features", crate_name);

    // Change directory to the crate's directory and run cargo check for the specific feature
    let status = Command::new("cargo")
        .current_dir(format!("packages/{}", crate_name)) // Set the current directory to the crate's directory
        .arg("check")
        .arg("--no-default-features")
        .status()
        .expect("Failed to execute cargo check");

    if !status.success() {
        println!(
            "Check failed in crate {} with no default features",
            crate_name
        );
        println!("cargo check -p {} --no-default-features", name);
        std::process::exit(1);
    }

    for (feature, _) in features.as_table().unwrap().iter() {
        // Skip special feature groups
        if feature == "default" || feature.ends_with("features") {
            continue;
        }

        println!(
            "Checking feature: {} in crate {} with default features",
            feature, crate_name
        );

        // Change directory to the crate's directory and run cargo check for the specific feature
        let status = Command::new("cargo")
            .current_dir(format!("packages/{}", crate_name)) // Set the current directory to the crate's directory
            .arg("check")
            .arg("--features")
            .arg(feature)
            .status()
            .expect("Failed to execute cargo check");

        if !status.success() {
            println!(
                "Feature check failed for feature: {} in crate {} with default features",
                feature, crate_name
            );
            println!("cargo check -p {} --features {}", name, feature);
            std::process::exit(1);
        }

        println!(
            "Checking feature: {} in crate {} with no default features",
            feature, crate_name
        );

        // Change directory to the crate's directory and run cargo check for the specific feature
        let status = Command::new("cargo")
            .current_dir(format!("packages/{}", crate_name)) // Set the current directory to the crate's directory
            .arg("check")
            .arg("--features")
            .arg(feature)
            .arg("--no-default-features")
            .status()
            .expect("Failed to execute cargo check");

        if !status.success() {
            println!(
                "Feature check failed for feature: {} in crate {} with no default features",
                feature, crate_name
            );
            println!(
                "cargo check -p {} --features {} --no-default-features",
                name, feature
            );
            std::process::exit(1);
        }
    }

    println!("All features checked successfully on {}", crate_name);
}
