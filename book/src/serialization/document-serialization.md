# Document Serialization

This chapter describes the binary wire format used to serialize and deserialize documents on Dash Platform. If you are building a tool that reads documents from gRPC responses, GroveDB storage, or any other source of raw document bytes, this is the specification you need.

Documents are **not self-describing**. The binary format does not include field names or type tags. To interpret the bytes, you must know which data contract and document type the document belongs to. The document type's schema defines the field order, types, and sizes that determine how the bytes map to values.

## High-level structure

Every serialized document follows this layout:

```text
┌──────────────────────┐
│  Serialization       │  varint (1-2 bytes)
│  Version             │  Currently: 0, 1, or 2
├──────────────────────┤
│  $id                 │  32 bytes
├──────────────────────┤
│  $ownerId            │  32 bytes
├──────────────────────┤
│  $creatorId          │  V2 only, if document type supports
│  (optional)          │  transfers/trading: 1 byte flag + 32 bytes
├──────────────────────┤
│  $revision           │  varint (if document type is mutable)
├──────────────────────┤
│  Time fields         │  2-byte bitfield + variable-length data
│  bitfield + data     │
├──────────────────────┤
│  Price               │  Only if document type supports trading
│  (optional)          │  1 byte flag + 8 bytes
├──────────────────────┤
│  User properties     │  Variable length, schema-dependent
│  (in schema order)   │
└──────────────────────┘
```

Source: `packages/rs-dpp/src/document/v0/serialize.rs`

## Serialization versions

The first bytes of a serialized document are a **varint** encoding the serialization version. This is not the protocol version or the document struct version — it is the version of the serialization format itself.

| Serialization Version | Description |
|----|-------------|
| 0 | Original format. All integers encoded as **i64** (8 bytes big-endian) regardless of their schema type. |
| 1 | Integers encoded at their **native size** (u8 = 1 byte, u16 = 2 bytes, u32 = 4 bytes, etc.). Otherwise identical to v0. |
| 2 | Same as v1, but adds **`$creatorId`** field after `$ownerId` for document types that support transfers or trading. |

The varint encoding uses the [`integer-encoding`](https://docs.rs/integer-encoding) crate's `VarInt` format. For values 0, 1, and 2, the varint is a single byte: `0x00`, `0x01`, or `0x02`.

```rust
// Serialization version is written first
let mut buffer: Vec<u8> = 2u64.encode_var_vec(); // version 2 → byte 0x02
```

When deserializing, the version varint is read first, then the appropriate deserialization logic is dispatched:

```rust
let serialized_version: u64 = serialized_document.read_varint()?;
match serialized_version {
    0 => DocumentV0::from_bytes_v0(serialized_document, document_type, platform_version),
    1 => DocumentV0::from_bytes_v1(serialized_document, document_type, platform_version),
    2 => DocumentV0::from_bytes_v2(serialized_document, document_type, platform_version),
    _ => Err(/* unknown version */),
}
```

Note: version 0 has a fallback — if deserialization as v0 (all i64) fails, it retries as v1 (native integer types). This handles edge cases from protocol versions 1–8 where the version byte was 0 but non-i64 integer types may have been used.

## Field-by-field breakdown

### `$id` (32 bytes)

The document's unique identifier, written as raw bytes. This is a 256-bit value derived from the contract ID, owner ID, document type name, and entropy via double SHA-256.

### `$ownerId` (32 bytes)

The identity that currently owns the document, written as raw bytes.

### `$creatorId` (v2 only, conditional)

Present only in serialization version 2, and only if the document type supports transfers (`documents_transferable`) or trading (`trade_mode != None`).

```text
0x01  [32 bytes creatorId]    — creator ID present
0x00                           — creator ID absent
```

### `$revision` (varint, conditional)

Present only if the document type requires revisions (mutable documents). Encoded as a varint (`u64`). New documents start at revision 1 (`INITIAL_REVISION`).

### Time fields (2-byte bitfield + data)

Time-related fields use a compact encoding with a **bitfield** to indicate which fields are present, followed by the data for each present field.

**Bitfield** (2 bytes, big-endian `u16`):

| Bit | Field |
|-----|-------|
| 0 (0x0001) | `$createdAt` |
| 1 (0x0002) | `$updatedAt` |
| 2 (0x0004) | `$transferredAt` |
| 3 (0x0008) | `$createdAtBlockHeight` |
| 4 (0x0010) | `$updatedAtBlockHeight` |
| 5 (0x0020) | `$transferredAtBlockHeight` |
| 6 (0x0040) | `$createdAtCoreBlockHeight` |
| 7 (0x0080) | `$updatedAtCoreBlockHeight` |
| 8 (0x0100) | `$transferredAtCoreBlockHeight` |

**Data**: For each bit that is set (in the order above), the corresponding value is appended:

- `$createdAt`, `$updatedAt`, `$transferredAt`: **8 bytes big-endian u64** — milliseconds since Unix epoch
- `$createdAtBlockHeight`, `$updatedAtBlockHeight`, `$transferredAtBlockHeight`: **8 bytes big-endian u64** — platform block height
- `$createdAtCoreBlockHeight`, `$updatedAtCoreBlockHeight`, `$transferredAtCoreBlockHeight`: **4 bytes big-endian u32** — core chain block height

For example, if a document has `$createdAt` and `$updatedAt` set, the bitfield would be `0x0003`, followed by 16 bytes (8 for each timestamp).

### Price (conditional)

If the document type's `trade_mode` allows seller-set pricing:

```text
0x01  [8 bytes big-endian u64]   — price in credits
0x00                              — no price set
```

### User-defined properties

Properties are serialized **in the order defined by the document type's schema** (`document_type.properties()` returns a `BTreeMap`, so properties are in alphabetical order by field name).

Each property is encoded based on its type and whether it is required:

**Required fields**: The value is written directly with no prefix byte.

**Optional fields**: A 1-byte presence flag is written first:
- `0x01` followed by the encoded value — field is present
- `0x00` — field is absent

#### Value encoding by type

All numeric values use **big-endian** byte order.

| Type | Encoding |
|------|----------|
| `u8` / `i8` | 1 byte |
| `u16` / `i16` | 2 bytes big-endian |
| `u32` / `i32` | 4 bytes big-endian |
| `u64` / `i64` | 8 bytes big-endian |
| `u128` / `i128` | 16 bytes big-endian |
| `f64` | 8 bytes big-endian IEEE 754 |
| `boolean` | 1 byte: `0x01` = true, `0x00` = false |
| `string` | varint length prefix + UTF-8 bytes |
| `byteArray` (fixed size) | raw bytes (no length prefix if min_size == max_size) |
| `byteArray` (variable size) | varint length prefix + raw bytes |
| `identifier` | 32 bytes raw |
| `date` | 8 bytes big-endian f64 |
| `object` | Nested fields serialized recursively in their schema order |

**Important**: In serialization version 0, all integer types are encoded as **i64** (8 bytes big-endian), regardless of the actual type in the schema. This means a `u8` field that should be 1 byte is encoded as 8 bytes in v0.

## Worked example: withdrawal document

The withdrawals contract defines a `withdrawal` document type with these properties (in alphabetical/BTreeMap order):

| Property | Type | Required |
|----------|------|----------|
| `amount` | integer (i64) | yes |
| `coreFeePerByte` | integer (i64) | yes |
| `outputScript` | byteArray (23–25 bytes, variable) | yes |
| `pooling` | integer (i64) | yes |
| `status` | integer (i64) | yes |
| `transactionIndex` | integer (i64) | no |
| `transactionSignHeight` | integer (i64) | no |

The document type requires `$createdAt`, `$updatedAt`, and `$revision`.

Here is a real serialized withdrawal document (hex), broken down byte by byte:

```text
02                                      ← serialization version (varint: 2)
0222 9eda 94b3 5be5 5ac2 22ca 8cc4      ← $id (32 bytes)
631c 0717 c9ee 4a22 3f2a 269e 06c9
a1be 7c54
36b3 e63b a54a ba9b 7599 4128 d124      ← $ownerId (32 bytes)
e9e1 cebe 348c d304 15b5 098c 6052
6de0 157e
c501                                    ← $revision (varint: 197)
0003                                    ← time bitfield: bits 0,1 set
                                          ($createdAt + $updatedAt)
0000019cd70f3323                        ← $createdAt: 1773134623523 ms
                                          (2026-03-10 09:23:43 UTC)
0000019d05406f0c                        ← $updatedAt: 1773909602060 ms
                                          (2026-03-19 08:40:02 UTC)
                                        ← user properties follow (schema-dependent)
```

To properly decode the properties section, you need the document type schema — field names, types, required flags, and order. This is why the `decode-document` CLI tool (in `packages/rs-scripts`) requires the contract and document type to be specified.

Using the tool on this document produces:

```text
id:         9LSAr59Fw7A1PHvX9WV1RWHCjL4PrijrHZpwhYDPkMq
owner_id:   4gY7wFM4o53jc8PJZ9KNzqzaJhXhPVMivJREKVwihKVF
created_at: 2026-03-10 09:23:43 UTC
updated_at: 2026-03-19 08:40:02 UTC
revision:   197

properties:
  amount: (i64)191000
  coreFeePerByte: (i64)1
  outputScript: bytes 76a914...88ac
  pooling: (i64)0
  status: (i64)2
  transactionIndex: (i64)9815
  transactionSignHeight: (i64)2440497
```

## The `decode-document` CLI tool

For convenience, the `rs-scripts` crate provides a `decode-document` binary that handles all of this deserialization automatically:

```bash
# Install
cargo install --path packages/rs-scripts

# Decode a withdrawal document from base64
decode-document -c withdrawals -d withdrawal "AgIintqUs1vl..."

# Decode from hex
decode-document -c withdrawals -d withdrawal "0202229eda94b35b..."

# Use a contract ID instead of a name
decode-document -c 4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6 -d withdrawal "..."
```

See `packages/rs-scripts/README.md` for full usage details.

## Common pitfalls for third-party deserializers

1. **The serialization version varint changed the layout.** If your code was written for version 0 or 1, version 2 documents will have different field offsets due to the `$creatorId` field. Always read the version varint first and branch accordingly.

2. **Integer encoding differs between v0 and v1+.** In version 0, a `u8` field occupies 8 bytes (encoded as i64). In version 1+, it occupies 1 byte. Parsing with the wrong version assumption will shift every subsequent field.

3. **The time fields bitfield is variable-length data.** The 2-byte bitfield tells you how many time fields follow. If you assume a fixed number of time fields, any document with a different set of time fields (e.g., one that includes `$transferredAt` or block heights) will be misaligned.

4. **Property order is alphabetical (BTreeMap order), not declaration order.** If you hard-code field offsets based on the JSON schema declaration order rather than alphabetical order, fields will be read from the wrong positions.

5. **Optional fields have a presence byte.** If you forget to read the `0x00`/`0x01` prefix for optional fields, every subsequent field will be shifted by one byte.

6. **ByteArray encoding depends on size constraints.** Fixed-size byte arrays (where `minItems == maxItems` in the schema) have no length prefix. Variable-size byte arrays have a varint length prefix. Check the schema to know which encoding is used.
