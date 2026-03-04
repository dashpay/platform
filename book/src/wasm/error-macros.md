# Error Macros

Dash Platform defines hundreds of consensus errors. Each one needs a WASM binding
so JavaScript code can inspect error codes, read messages, and serialize errors for
transport. Writing a wrapper struct for every single error by hand would be tedious,
error-prone, and a maintenance burden.

This chapter covers the `generic_consensus_error!` macro that generates WASM bindings
automatically, the `paste!` crate that makes it work, the manual binding pattern for
errors that need custom methods, and how the two approaches coexist.

## The Problem

Consider Platform's error hierarchy. At the top level:

```
ConsensusError
  BasicError (dozens of variants)
  StateError (dozens of variants)
  SignatureError (handful of variants)
  FeeError (one variant)
```

Each variant wraps a specific error struct -- `TransitionNoInputsError`,
`DocumentNotFoundError`, `MasternodeNotFoundError`, and so on. There are well over a
hundred of these.

Every one needs a JavaScript class with:

- A `getCode()` method returning the numeric error code.
- A `message` getter returning the human-readable error string.
- A `serialize()` method for wire encoding.
- A `From<&RustType>` implementation for conversion.

Writing all of this for each error would mean hundreds of nearly identical files.

## The generic_consensus_error! Macro

The macro lives in `packages/wasm-dpp/src/errors/generic_consensus_error.rs`:

```rust
#[macro_export]
macro_rules! generic_consensus_error {
    ($error_type:ident, $error_instance:expr) => {{
        use {
            dpp::{
                consensus::{codes::ErrorWithCode, ConsensusError},
                serialization::PlatformSerializableWithPlatformVersion,
                version::PlatformVersion,
            },
            paste::paste,
            wasm_bindgen::prelude::wasm_bindgen,
            $crate::buffer::Buffer,
        };

        paste! {
            #[derive(Debug)]
            #[wasm_bindgen(js_name=$error_type)]
            pub struct [<$error_type Wasm>] {
                inner: $error_type
            }

            impl From<&$error_type> for [<$error_type Wasm>] {
                fn from(e: &$error_type) -> Self {
                    Self {
                        inner: e.clone()
                    }
                }
            }

            #[wasm_bindgen(js_class=$error_type)]
            impl [<$error_type Wasm>] {
                #[wasm_bindgen(js_name=getCode)]
                pub fn get_code(&self) -> u32 {
                    ConsensusError::from(self.inner.clone()).code()
                }

                #[wasm_bindgen(getter)]
                pub fn message(&self) -> String {
                    self.inner.to_string()
                }

                pub fn serialize(&self) -> Result<Buffer, JsError> {
                    let bytes = ConsensusError::from(self.inner.clone())
                        .serialize_to_bytes_with_platform_version(
                            PlatformVersion::first(),
                        )
                        .map_err(JsError::from)?;

                    Ok(Buffer::from_bytes(bytes.as_slice()))
                }
            }

            [<$error_type Wasm>]::from($error_instance)
        }
    }};
}
```

### What It Generates

For a call like `generic_consensus_error!(MasternodeNotFoundError, e)`, the macro
generates:

1. **A wrapper struct**: `MasternodeNotFoundErrorWasm` with `inner: MasternodeNotFoundError`
2. **A From impl**: `From<&MasternodeNotFoundError> for MasternodeNotFoundErrorWasm`
3. **Three methods**:
   - `get_code()` -- returns the numeric error code
   - `message` -- returns the `Display` string
   - `serialize()` -- encodes the error to bytes
4. **An instantiation**: Creates a `MasternodeNotFoundErrorWasm` from the error reference

### The paste! Crate

The magic of `[<$error_type Wasm>]` comes from the `paste` crate. Standard Rust
macros cannot concatenate identifiers -- you cannot write `$error_type ## Wasm` to
create a new identifier. The `paste!` macro provides this:

```rust
paste! {
    pub struct [<$error_type Wasm>] { ... }
    //         ^^^^^^^^^^^^^^^^^
    //         pastes to: MasternodeNotFoundErrorWasm
}
```

Inside `paste! { ... }`, `[<token1 token2>]` concatenates tokens into a single
identifier. This is what allows the macro to generate both the JavaScript name
(`MasternodeNotFoundError` via `js_name`) and the Rust struct name
(`MasternodeNotFoundErrorWasm` via paste).

## How It Is Used

The macro is used inline within the `from_consensus_error_ref` function and its
helpers in `packages/wasm-dpp/src/errors/consensus/consensus_error.rs`. Here is
a representative excerpt:

```rust
pub fn from_state_error(state_error: &StateError) -> JsValue {
    match state_error {
        // Manual wrappers (have custom methods)
        StateError::DocumentAlreadyPresentError(e) => {
            DocumentAlreadyPresentErrorWasm::from(e).into()
        }
        StateError::DocumentNotFoundError(e) => {
            DocumentNotFoundErrorWasm::from(e).into()
        }

        // Macro-generated wrappers (standard interface only)
        StateError::MasternodeNotFoundError(e) => {
            generic_consensus_error!(MasternodeNotFoundError, e).into()
        }
        StateError::DocumentContestCurrentlyLockedError(e) => {
            generic_consensus_error!(
                DocumentContestCurrentlyLockedError, e
            ).into()
        }
        StateError::TokenIsPausedError(e) => {
            generic_consensus_error!(TokenIsPausedError, e).into()
        }
        // ... dozens more
    }
}
```

The pattern is clear: use the macro for errors that need only `getCode()`, `message`,
and `serialize()`. Use manual wrappers for errors that need custom accessors.

## Manual Wrappers: When You Need More

Some errors expose domain-specific data that JavaScript code needs to access. For
these, you write a full wrapper by hand. Here is `DataContractMaxDepthExceedError`
from `packages/wasm-dpp/src/errors/consensus/basic/data_contract/`:

```rust
#[wasm_bindgen(js_name=DataContractMaxDepthExceedError)]
pub struct DataContractMaxDepthExceedErrorWasm {
    inner: DataContractMaxDepthExceedError,
}

impl From<&DataContractMaxDepthExceedError>
    for DataContractMaxDepthExceedErrorWasm
{
    fn from(e: &DataContractMaxDepthExceedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractMaxDepthError)]
impl DataContractMaxDepthExceedErrorWasm {
    #[wasm_bindgen(js_name=getMaxDepth)]
    pub fn get_max_depth(&self) -> usize {
        self.inner.max_depth()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
```

The custom `get_max_depth()` method lets JavaScript inspect the specific limit that
was exceeded. The macro cannot generate these domain-specific accessors -- it only
knows about the three standard methods.

## The from_dpp_err Pattern

At the top level, Rust's `ProtocolError` is converted to a `JsValue` through
`from_dpp_err` in `packages/wasm-dpp/src/errors/from.rs`:

```rust
pub fn from_dpp_err(pe: ProtocolError) -> JsValue {
    match pe {
        ProtocolError::ConsensusError(consensus_error) => {
            from_consensus_error(*consensus_error)
        }
        ProtocolError::DataContractError(e) => {
            from_data_contract_to_js_error(e)
        }
        ProtocolError::Document(e) => {
            from_document_to_js_error(*e)
        }
        ProtocolError::DataContractNotPresentError(err) => {
            DataContractNotPresentNotConsensusErrorWasm::new(
                err.data_contract_id()
            ).into()
        }
        ProtocolError::ValueError(value_error) => {
            PlatformValueErrorWasm::from(value_error).into()
        }
        _ => JsValue::from_str(
            &format!("Error conversion not implemented: {pe:#}")
        ),
    }
}
```

This is the entry point for error conversion. It dispatches to the appropriate
conversion function based on the error variant. The fallback case converts
unhandled errors to a string -- not ideal, but it ensures no errors are silently
swallowed.

## The Consensus Error Dispatch

The `from_consensus_error_ref` function dispatches across the entire consensus
error hierarchy:

```rust
pub fn from_consensus_error_ref(e: &DPPConsensusError) -> JsValue {
    match e {
        DPPConsensusError::FeeError(e) => match e {
            FeeError::BalanceIsNotEnoughError(e) =>
                BalanceIsNotEnoughErrorWasm::from(e).into(),
        },
        DPPConsensusError::SignatureError(e) =>
            from_signature_error(e),
        DPPConsensusError::StateError(state_error) =>
            from_state_error(state_error),
        DPPConsensusError::BasicError(basic_error) =>
            from_basic_error(basic_error),
        DPPConsensusError::DefaultError =>
            JsError::new("DefaultError").into(),
    }
}
```

Each sub-function (`from_state_error`, `from_basic_error`, `from_signature_error`)
handles its category, using either manual wrappers or the macro as appropriate.

## Adding a New WASM Error Binding

When a new consensus error is added to `rs-dpp`, you need to add its WASM binding.
Here is the checklist:

**If the error only needs `getCode()`, `message`, and `serialize()`:**

1. Import the error type in `consensus_error.rs`.
2. Add a match arm using the macro:

```rust
StateError::YourNewError(e) => {
    generic_consensus_error!(YourNewError, e).into()
}
```

That is it. The macro handles everything else.

**If the error needs custom accessors:**

1. Create a new file in the appropriate subdirectory under
   `packages/wasm-dpp/src/errors/consensus/`.
2. Define the wrapper struct, `From` impl, and methods following the manual pattern.
3. Add the wrapper to the `mod.rs` file.
4. Import it in `consensus_error.rs`.
5. Add a match arm using the manual wrapper:

```rust
StateError::YourNewError(e) => {
    YourNewErrorWasm::from(e).into()
}
```

## Rules

**Do:**

- Use `generic_consensus_error!` for errors that need only the standard three methods.
- Use manual wrappers when JavaScript needs to access error-specific fields.
- Follow the existing file organization: one file per manual error, grouped by category.
- Always provide `getCode()`, `message`, and `serialize()` -- these are the standard interface.
- Test that new errors convert correctly in both directions.

**Don't:**

- Write manual wrappers when the macro would suffice -- it just creates maintenance debt.
- Forget to add the match arm in `consensus_error.rs` -- unhandled errors will fall
  through to the `DefaultError` case or an `unreachable_patterns` guard.
- Use `js_name` values that differ from the Rust error type name -- JavaScript
  developers should see the same name they find in documentation.
- Skip the `serialize()` method -- it is needed for error transport across process boundaries.
- Modify `generic_consensus_error!` without understanding that every call site will
  be affected -- it generates code in every match arm that uses it.
