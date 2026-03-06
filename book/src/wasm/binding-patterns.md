# Binding Patterns

Dash Platform's core logic is written in Rust. But many developers build applications
in JavaScript -- browser-based wallets, Node.js services, React Native apps. The
`wasm-dpp` package bridges these worlds by compiling Rust types to WebAssembly and
exposing them as JavaScript classes.

This chapter covers the patterns used to create these bindings: the wrapper struct
pattern, naming conventions, buffer handling at the boundary, getter/setter patterns,
and the `Inner` trait.

## The Problem

Rust and JavaScript have fundamentally different type systems. Rust has ownership,
lifetimes, and zero-cost abstractions. JavaScript has garbage collection, prototype
chains, and dynamic types. `wasm-bindgen` bridges the gap, but it requires careful
manual work to make the resulting JavaScript API feel natural.

The core challenge: you have a Rust type like `Identity` with methods like `id()`,
`balance()`, and `public_keys()`. You need to expose it to JavaScript as a class
with methods like `getId()`, `getBalance()`, and `getPublicKeys()` -- following
JavaScript naming conventions while maintaining Rust's safety guarantees.

## The Wrapper Struct Pattern

Every Rust type exposed to JavaScript gets a wrapper struct. Here is `IdentityWasm`
from `packages/wasm-dpp/src/identity/identity.rs`:

```rust
#[wasm_bindgen(js_name=Identity)]
#[derive(Clone)]
pub struct IdentityWasm {
    inner: Identity,
    metadata: Option<Metadata>,
}
```

The pattern has three parts:

1. **`#[wasm_bindgen(js_name=Identity)]`** -- tells `wasm_bindgen` to expose this
   struct as `Identity` in JavaScript, not `IdentityWasm`.
2. **`inner: Identity`** -- the real Rust type, hidden from JavaScript.
3. **Additional fields** -- any extra state needed at the WASM boundary (like
   `metadata` here, which is managed separately in JS).

### Why a Wrapper?

You cannot put `#[wasm_bindgen]` directly on `Identity` for several reasons:

- `Identity` is defined in `rs-dpp`, a different crate. You cannot add attributes
  to types in other crates.
- `Identity` may contain types that `wasm_bindgen` cannot handle (nested enums,
  complex generics, trait objects).
- The JavaScript API should have camelCase methods (`getId`), not Rust-style
  snake_case (`id`).
- Some conversions (like turning `Identifier` into a `Buffer`) only make sense at
  the WASM boundary.

## From/Into Conversions

Every wrapper implements bidirectional conversion:

```rust
impl From<IdentityWasm> for Identity {
    fn from(identity: IdentityWasm) -> Self {
        identity.inner
    }
}

impl From<Identity> for IdentityWasm {
    fn from(identity: Identity) -> Self {
        Self {
            inner: identity,
            metadata: None,
        }
    }
}
```

This lets internal Rust code work with the real `Identity` type while the WASM
boundary works with `IdentityWasm`. The conversion is zero-cost -- it just moves
the inner value.

## JavaScript Method Naming

Methods use `#[wasm_bindgen(js_name=...)]` to follow JavaScript conventions:

```rust
#[wasm_bindgen(js_class=Identity)]
impl IdentityWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.inner.id().into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: IdentifierWrapper) {
        self.inner.set_id(id.into());
    }

    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> u64 {
        self.inner.balance()
    }

    #[wasm_bindgen(js_name=setBalance)]
    pub fn set_balance(&mut self, balance: u64) {
        self.inner.set_balance(balance);
    }
}
```

Note the `#[wasm_bindgen(js_class=Identity)]` on the `impl` block -- this associates
the methods with the `Identity` JavaScript class (the `js_name` from the struct).

In JavaScript, these become:

```javascript
const identity = new Identity(platformVersion);
const id = identity.getId();
identity.setBalance(1000n);
```

## Getter Properties

For simple values, you can use JavaScript getter syntax:

```rust
#[wasm_bindgen(getter)]
pub fn balance(&self) -> u64 {
    self.inner.balance()
}
```

In JavaScript, this becomes a property access:

```javascript
const bal = identity.balance;  // no parentheses
```

Platform uses both patterns -- `getBalance()` method and `balance` getter -- for
the same value. This provides flexibility: the getter is concise for reading, the
method is consistent for tools that expect a getter/setter pair.

## Constructors

The `#[wasm_bindgen(constructor)]` attribute creates a JavaScript constructor:

```rust
#[wasm_bindgen(constructor)]
pub fn new(platform_version: u32) -> Result<IdentityWasm, JsValue> {
    let platform_version = &PlatformVersion::get(platform_version)
        .map_err(|e| JsValue::from(e.to_string()))?;

    Identity::default_versioned(platform_version)
        .map(Into::into)
        .map_err(from_dpp_err)
}
```

Notice the error handling: Rust `Result` becomes a JavaScript throw. The
`from_dpp_err` function converts Rust `ProtocolError` into JavaScript error objects.

## Buffer Handling at the Boundary

Binary data crosses the WASM boundary as `Buffer` (a custom type that wraps
`Uint8Array`):

```rust
#[wasm_bindgen(js_name=toBuffer)]
pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
    let bytes = PlatformSerializable::serialize_to_bytes(
        &self.inner.clone()
    ).with_js_error()?;
    Ok(Buffer::from_bytes(&bytes))
}

#[wasm_bindgen(js_name=fromBuffer)]
pub fn from_buffer(buffer: Vec<u8>) -> Result<IdentityWasm, JsValue> {
    let identity: Identity =
        PlatformDeserializable::deserialize_from_bytes(buffer.as_slice())
            .with_js_error()?;
    Ok(identity.into())
}
```

The `toBuffer/fromBuffer` pair is the standard serialization interface. Every WASM
type that needs to be stored or transmitted implements this pair.

## Handling Complex Types: Arrays and Objects

JavaScript arrays and objects require special handling. For collections of public keys:

```rust
#[wasm_bindgen(js_name=getPublicKeys)]
pub fn get_public_keys(&self) -> Vec<JsValue> {
    self.inner
        .public_keys()
        .values()
        .cloned()
        .map(IdentityPublicKeyWasm::from)  // Rust -> Wrapper
        .map(JsValue::from)               // Wrapper -> JsValue
        .collect()
}

#[wasm_bindgen(js_name=setPublicKeys)]
pub fn set_public_keys(&mut self, public_keys: js_sys::Array)
    -> Result<usize, JsValue>
{
    if public_keys.length() == 0 {
        return Err("Must use array of PublicKeys".into());
    }

    let public_keys = public_keys
        .iter()
        .map(|key| {
            key.to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")
                .map(|key| {
                    let key = IdentityPublicKey::from(key.to_owned());
                    (key.id(), key)
                })
        })
        .collect::<Result<_, _>>()?;

    self.inner.set_public_keys(public_keys);
    Ok(self.inner.public_keys().len())
}
```

The `to_wasm::<T>("TypeName")` helper extracts a Rust wrapper from a `JsValue`,
validating that it is the correct type.

## JSON Serialization

Most WASM types provide `toJSON` and `toObject` methods:

```rust
#[wasm_bindgen(js_name=toJSON)]
pub fn to_json(&self) -> Result<JsValue, JsValue> {
    let mut value = self.inner.to_object().with_js_error()?;

    // Convert identifiers to Base58 strings for readability
    value.replace_at_paths(
        dpp::identity::IDENTIFIER_FIELDS_RAW_OBJECT,
        ReplacementType::TextBase58,
    ).map_err(|e| e.to_string())?;

    // Convert binary key data to Base64
    let public_keys = value
        .get_array_mut_ref(dpp::identity::property_names::PUBLIC_KEYS)
        .map_err(|e| e.to_string())?;

    for key in public_keys.iter_mut() {
        key.replace_at_paths(
            dpp::identity::identity_public_key::BINARY_DATA_FIELDS,
            ReplacementType::TextBase64,
        ).map_err(|e| e.to_string())?;
    }

    let json = value.try_into_validating_json()
        .map_err(|e| e.to_string())?
        .to_string();

    js_sys::JSON::parse(&json)
}
```

The `toJSON` method applies human-readable encoding (Base58 for identifiers, Base64
for binary data) before converting to a JavaScript object. This is the format used
in APIs and debugging tools.

## The Inner Trait

To standardize wrapper access, Platform defines an `Inner` trait:

```rust
impl Inner for IdentityWasm {
    type InnerItem = Identity;

    fn into_inner(self) -> Self::InnerItem {
        self.inner
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.inner
    }
}
```

This trait provides a consistent way for other Rust code in the WASM layer to access
the unwrapped type without knowing the wrapper's internal structure.

## Rules

**Do:**

- Follow the wrapper struct pattern: `TypeWasm { inner: Type }`.
- Use `js_name` on the struct and `js_class` on the `impl` block.
- Implement `From<Type> for TypeWasm` and `From<TypeWasm> for Type`.
- Use `Buffer::from_bytes` for binary data crossing the boundary.
- Implement `toBuffer/fromBuffer` for any type that needs serialization.
- Use `from_dpp_err` or `with_js_error()` for error conversion.
- Implement the `Inner` trait for consistent wrapper access.

**Don't:**

- Expose Rust types directly to WASM -- always wrap them.
- Use Rust naming conventions (`get_id`) in the JavaScript API -- use `js_name=getId`.
- Return `Result<T, ProtocolError>` from WASM methods -- convert to `Result<T, JsValue>`.
- Forget to handle empty arrays and invalid inputs with clear error messages.
- Expose internal fields that do not make sense in JavaScript (like `Arc` or `Mutex`).
