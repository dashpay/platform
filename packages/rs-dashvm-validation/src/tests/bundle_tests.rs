use super::{latest_profile, profile_with, wasm, MINIMAL};
use crate::bundle::{BundleBinding, FuncSignature, ModuleName, ValueType};
use crate::bundle_preparation::{validate_and_prepare_bundle, BundleInput, DeclaredBinding};
use crate::errors::{BundleError, ImportRejection, ModuleError, PreparationError};

fn name(value: &str) -> ModuleName {
    ModuleName::parse(value, 64).expect("valid")
}

/// A module that exports `helper: (i32) -> i32` besides the required exports.
fn provider() -> Vec<u8> {
    wasm(
        r#"(module
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
  (func (export "helper") (param i32) (result i32) local.get 0)
)"#,
    )
}

/// A module that imports `helper` from `target` with the given signature text.
fn consumer(target: &str, signature: &str) -> Vec<u8> {
    wasm(&format!(
        r#"(module
  (import "dash:{target}" "helper" (func $h {signature}))
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
)"#
    ))
}

fn bundle_error(error: PreparationError) -> BundleError {
    match error {
        PreparationError::Bundle(error) => error,
        PreparationError::Module { name, source } => {
            panic!("expected a bundle error, module `{name}` failed with {source:?}")
        }
    }
}

#[test]
fn should_prepare_a_two_module_bundle_and_order_dependencies_first() {
    let profile = latest_profile();
    let app = consumer("lib", "(param i32) (result i32)");
    let lib = provider();
    let bundle = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app",
                canonical_bytes: &app,
            },
            BundleInput {
                name: "lib",
                canonical_bytes: &lib,
            },
        ],
        &[DeclaredBinding {
            importer: "app",
            target: "lib",
            export: "helper",
        }],
        &[("app", "run")],
        &profile,
    )
    .expect("prepared");
    assert_eq!(
        bundle
            .modules
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["app", "lib"]
    );
    assert_eq!(
        bundle.bindings,
        vec![BundleBinding {
            importer: name("app"),
            target: name("lib"),
            export: "helper".to_owned(),
            signature: FuncSignature::new(vec![ValueType::I32], vec![ValueType::I32]),
        }]
    );
    assert_eq!(bundle.initialization_order, vec![name("lib"), name("app")]);
    assert_eq!(bundle.dependencies_of(&name("app")), vec![name("lib")]);
    assert!(bundle.dependencies_of(&name("lib")).is_empty());
    assert_eq!(bundle.entries.len(), 1);
    assert_eq!(bundle.preparation_generation, profile.generation);
    assert_eq!(bundle.metering_generation, profile.metering);
    assert!(bundle.module(&name("lib")).is_some());
    assert!(bundle.module(&name("zzz")).is_none());
}

#[test]
fn should_order_independent_modules_by_canonical_name() {
    let profile = latest_profile();
    let a = wasm(MINIMAL);
    let inputs: Vec<BundleInput<'_>> = ["c", "a", "b"]
        .iter()
        .map(|name| BundleInput {
            name,
            canonical_bytes: &a,
        })
        .collect();
    let bundle =
        validate_and_prepare_bundle(&inputs, &[], &[("b", "run")], &profile).expect("prepared");
    assert_eq!(
        bundle.initialization_order,
        vec![name("a"), name("b"), name("c")]
    );
}

#[test]
fn should_reject_a_dependency_cycle() {
    let profile = latest_profile();
    // a imports from b, b imports from a; both export helper.
    let a = wasm(
        r#"(module
  (import "dash:b" "helper" (func (param i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
  (func (export "helper") (param i32) (result i32) local.get 0)
)"#,
    );
    let b = wasm(
        r#"(module
  (import "dash:a" "helper" (func (param i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
  (func (export "helper") (param i32) (result i32) local.get 0)
)"#,
    );
    let c = wasm(MINIMAL);
    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "a",
                canonical_bytes: &a,
            },
            BundleInput {
                name: "b",
                canonical_bytes: &b,
            },
            BundleInput {
                name: "c",
                canonical_bytes: &c,
            },
        ],
        &[
            DeclaredBinding {
                importer: "a",
                target: "b",
                export: "helper",
            },
            DeclaredBinding {
                importer: "b",
                target: "a",
                export: "helper",
            },
        ],
        &[("c", "run")],
        &profile,
    )
    .expect_err("cycle");
    assert_eq!(
        bundle_error(error),
        BundleError::DependencyCycle {
            modules: vec![name("a"), name("b")]
        }
    );
}

#[test]
fn should_reject_a_binding_to_an_unknown_module() {
    let profile = latest_profile();
    let app = consumer("lib", "(param i32) (result i32)");
    let error = validate_and_prepare_bundle(
        &[BundleInput {
            name: "app",
            canonical_bytes: &app,
        }],
        &[DeclaredBinding {
            importer: "app",
            target: "lib",
            export: "helper",
        }],
        &[("app", "run")],
        &profile,
    )
    .expect_err("unknown target");
    assert_eq!(
        bundle_error(error),
        BundleError::UnknownBindingTarget {
            importer: name("app"),
            target: name("lib")
        }
    );
}

#[test]
fn should_reject_a_binding_to_a_missing_export_and_a_signature_mismatch() {
    let profile = latest_profile();
    let lib = provider();
    let missing = wasm(
        r#"(module
  (import "dash:lib" "nope" (func (param i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
)"#,
    );
    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app",
                canonical_bytes: &missing,
            },
            BundleInput {
                name: "lib",
                canonical_bytes: &lib,
            },
        ],
        &[DeclaredBinding {
            importer: "app",
            target: "lib",
            export: "nope",
        }],
        &[("app", "run")],
        &profile,
    )
    .expect_err("missing export");
    assert_eq!(
        bundle_error(error),
        BundleError::MissingBindingExport {
            importer: name("app"),
            target: name("lib"),
            export: "nope".to_owned()
        }
    );

    let mismatched = consumer("lib", "(param i64) (result i32)");
    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app",
                canonical_bytes: &mismatched,
            },
            BundleInput {
                name: "lib",
                canonical_bytes: &lib,
            },
        ],
        &[DeclaredBinding {
            importer: "app",
            target: "lib",
            export: "helper",
        }],
        &[("app", "run")],
        &profile,
    )
    .expect_err("signature mismatch");
    assert_eq!(
        bundle_error(error),
        BundleError::BindingSignature {
            importer: name("app"),
            target: name("lib"),
            export: "helper".to_owned(),
            imported: Box::new(FuncSignature::new(
                vec![ValueType::I64],
                vec![ValueType::I32]
            )),
            exported: Box::new(FuncSignature::new(
                vec![ValueType::I32],
                vec![ValueType::I32]
            )),
        }
    );
}

#[test]
fn should_require_declared_bindings_to_match_the_code() {
    let profile = latest_profile();
    let app = consumer("lib", "(param i32) (result i32)");
    let lib = provider();
    let inputs = [
        BundleInput {
            name: "app",
            canonical_bytes: &app,
        },
        BundleInput {
            name: "lib",
            canonical_bytes: &lib,
        },
    ];
    let undeclared = validate_and_prepare_bundle(&inputs, &[], &[("app", "run")], &profile)
        .expect_err("undeclared binding");
    assert!(matches!(
        bundle_error(undeclared),
        BundleError::BindingsMismatch { detail } if detail.contains("not declared")
    ));
    let overdeclared = validate_and_prepare_bundle(
        &inputs,
        &[
            DeclaredBinding {
                importer: "app",
                target: "lib",
                export: "helper",
            },
            DeclaredBinding {
                importer: "lib",
                target: "app",
                export: "run",
            },
        ],
        &[("app", "run")],
        &profile,
    )
    .expect_err("overdeclared binding");
    assert!(matches!(
        bundle_error(overdeclared),
        BundleError::BindingsMismatch { detail } if detail.contains("not imported by the code")
    ));
}

#[test]
fn should_reject_bad_names_duplicates_and_bad_counts() {
    let profile = latest_profile();
    let bytes = wasm(MINIMAL);
    let error = validate_and_prepare_bundle(
        &[BundleInput {
            name: "App",
            canonical_bytes: &bytes,
        }],
        &[],
        &[("App", "run")],
        &profile,
    )
    .expect_err("bad name");
    assert!(matches!(
        bundle_error(error),
        BundleError::InvalidModuleName { .. }
    ));

    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app",
                canonical_bytes: &bytes,
            },
            BundleInput {
                name: "app",
                canonical_bytes: &bytes,
            },
        ],
        &[],
        &[("app", "run")],
        &profile,
    )
    .expect_err("duplicate");
    assert_eq!(
        bundle_error(error),
        BundleError::DuplicateModuleName { name: name("app") }
    );

    let error = validate_and_prepare_bundle(&[], &[], &[], &profile).expect_err("empty");
    assert_eq!(
        bundle_error(error),
        BundleError::ModuleCount {
            actual: 0,
            max: profile.limits.max_modules_per_bundle
        }
    );

    let small = profile_with(|limits| limits.max_modules_per_bundle = 1);
    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "a",
                canonical_bytes: &bytes,
            },
            BundleInput {
                name: "b",
                canonical_bytes: &bytes,
            },
        ],
        &[],
        &[("a", "run")],
        &small,
    )
    .expect_err("too many");
    assert_eq!(
        bundle_error(error),
        BundleError::ModuleCount { actual: 2, max: 1 }
    );
    validate_and_prepare_bundle(
        &[BundleInput {
            name: "a",
            canonical_bytes: &bytes,
        }],
        &[],
        &[("a", "run")],
        &small,
    )
    .expect("at the module cap");
}

#[test]
fn should_check_entries() {
    let profile = latest_profile();
    let bytes = wasm(MINIMAL);
    let inputs = [BundleInput {
        name: "app",
        canonical_bytes: &bytes,
    }];
    assert_eq!(
        bundle_error(
            validate_and_prepare_bundle(&inputs, &[], &[], &profile).expect_err("no entries")
        ),
        BundleError::NoEntries
    );
    assert_eq!(
        bundle_error(
            validate_and_prepare_bundle(&inputs, &[], &[("other", "run")], &profile)
                .expect_err("unknown module")
        ),
        BundleError::EntryModuleUnknown {
            module: name("other"),
            export: "run".to_owned()
        }
    );
    assert_eq!(
        bundle_error(
            validate_and_prepare_bundle(&inputs, &[], &[("app", "dash_alloc")], &profile)
                .expect_err("wrong signature")
        ),
        BundleError::EntryNotFound {
            module: name("app"),
            export: "dash_alloc".to_owned()
        }
    );
    assert_eq!(
        bundle_error(
            validate_and_prepare_bundle(&inputs, &[], &[("app", "run"), ("app", "run")], &profile)
                .expect_err("duplicate")
        ),
        BundleError::DuplicateEntry {
            module: name("app"),
            export: "run".to_owned()
        }
    );
}

#[test]
fn should_reject_a_dash_vm_import_inside_a_bundle_as_a_module_error() {
    let profile = latest_profile();
    let claims = wasm(
        r#"(module
  (import "dash_vm" "stack_bytes" (global (mut i32)))
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
)"#,
    );
    let lib = provider();
    let error = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app",
                canonical_bytes: &claims,
            },
            BundleInput {
                name: "lib",
                canonical_bytes: &lib,
            },
        ],
        &[],
        &[("lib", "run")],
        &profile,
    )
    .expect_err("refused");
    assert_eq!(
        error,
        PreparationError::Module {
            name: name("app"),
            source: ModuleError::Import {
                module: "dash_vm".to_owned(),
                name: "stack_bytes".to_owned(),
                reason: ImportRejection::ReservedModule,
            },
        }
    );
}

#[test]
fn should_compute_a_stable_digest_that_covers_names_bindings_and_entries() {
    let profile = latest_profile();
    let app = consumer("lib", "(param i32) (result i32)");
    let lib = provider();
    let prepare = |entries: &[(&str, &str)]| {
        validate_and_prepare_bundle(
            &[
                BundleInput {
                    name: "app",
                    canonical_bytes: &app,
                },
                BundleInput {
                    name: "lib",
                    canonical_bytes: &lib,
                },
            ],
            &[DeclaredBinding {
                importer: "app",
                target: "lib",
                export: "helper",
            }],
            entries,
            &profile,
        )
        .expect("prepared")
    };
    let one = prepare(&[("app", "run")]);
    let again = prepare(&[("app", "run")]);
    assert_eq!(one.digest, again.digest, "same inputs, same digest");
    let more_entries = prepare(&[("app", "run"), ("lib", "run")]);
    assert_ne!(one.digest, more_entries.digest, "entries are covered");
    let renamed = validate_and_prepare_bundle(
        &[
            BundleInput {
                name: "app2",
                canonical_bytes: &app,
            },
            BundleInput {
                name: "lib",
                canonical_bytes: &lib,
            },
        ],
        &[DeclaredBinding {
            importer: "app2",
            target: "lib",
            export: "helper",
        }],
        &[("app2", "run")],
        &profile,
    )
    .expect("prepared");
    assert_ne!(one.digest, renamed.digest, "names are covered");
    // Per-module hashes are independent of the bundle: lib is the same module in both.
    assert_eq!(
        one.module(&name("lib")).map(|m| m.prepared_hash),
        renamed.module(&name("lib")).map(|m| m.prepared_hash)
    );
}
