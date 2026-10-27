# Contract-Level Keys and config

The document types sit inside a data contract, which has keys of its own: its id, its owner, its version, the shared definitions its types may point at, its tokens and groups, and a search description. One of them, `config`, holds contract-wide settings: whether the contract can ever change, whether it keeps its history, the defaults its document types inherit, how integers are stored, and whether and how it is moderated. Almost every `config` setting is chosen once, at registration, and kept for the contract's life.

## Example

```json
{
  "$formatVersion": "1",
  "id": "AY6xWncZUFv2GCrS5seqKthUfbW9yYyUXtF8diSuHQ4g",
  "ownerId": "AtirhSVpAWF7dEt6dLAmesC4Sr1MsJ9bFC1nLAoNnq2S",
  "version": 1,
  "config": {
    "$formatVersion": "2",
    "canBeDeleted": false,
    "readonly": false,
    "keepsHistory": false,
    "documentsKeepHistoryContractDefault": false,
    "documentsMutableContractDefault": true,
    "documentsCanBeDeletedContractDefault": true,
    "sizedIntegerTypes": true,
    "moderation": {
      "banlist": true,
      "suspensions": true,
      "moderators": { "$type": "contractOwner" }
    }
  },
  "documentSchemas": {
    "post": {
      "type": "object",
      "properties": {
        "text": { "type": "string", "minLength": 1, "maxLength": 280, "position": 0 }
      },
      "required": ["text"],
      "additionalProperties": false
    }
  },
  "keywords": ["social", "microblog"],
  "description": "Short public posts"
}
```

A first version of a small social contract. Its config states the defaults explicitly, stores integers in the smallest width their bounds allow, and keeps a banlist and a suspension list that the owner edits. `keywords` and `description` make it findable through the keyword search contract.

## Contract keys

| Key | Value | What it is | Since |
|---|---|---|---|
| `$formatVersion` | `"0"` or `"1"` | The contract's serialization format. Format 1 is the default from protocol version 9 and is the one that carries `groups`, `tokens`, `keywords`, `description` and the timestamps. | 1 |
| `id` | identifier | The contract's id: a hash of `ownerId` and the identity nonce of the create transition. A create whose id is not that hash is refused (`InvalidDataContractIdError`, 10204). | 1 |
| `ownerId` | identifier | The identity that registers the contract, and the only one that can update it. | 1 |
| `version` | integer | 1 when the contract is created; each update must raise it by exactly one (`InvalidDataContractVersionError`, 10212). | 1 |
| `config` | object | Contract-wide settings: see [`config`](#config) below. Absent means the defaults. | 1 |
| `documentSchemas` | object | The document types, by name. See [`documentSchemas`](#documentschemas). | 1 |
| `schemaDefs` | object | Definitions every document type may point at with `$ref`. An update may add definitions, not remove them (`IncompatibleDataContractSchemaError`, 10213). | 1 |
| `groups` | object | Groups of identities that act together, each member with a voting power, whose approval some token actions need. See [Data Contracts](../data-model/data-contracts.md#what-v1-added). Not the same thing as a [contract group](../data-model/contract-groups.md), a set of contracts. | 9 |
| `tokens` | object | The contract's tokens, keyed by position `0`, `1`, and so on. Document types may charge them with [`tokenCost`](token-cost.md). | 9 |
| `keywords` | array of strings | Search keywords. See [`keywords` and `description`](#keywords-and-description). | 9 |
| `description` | string | A short description for search. See [`keywords` and `description`](#keywords-and-description). | 9 |
| `createdAt`, `updatedAt`, `createdAtBlockHeight`, `updatedAtBlockHeight`, `createdAtEpoch`, `updatedAtEpoch` | numbers | When the contract was created and last updated. The platform sets them; a contract does not write them. | 9 |

### `documentSchemas`

| | |
|---|---|
| **Where** | contract |
| **Value** | object mapping each document type name to its schema |
| **Since** | protocol version 1 |
| **On update** | Document types may be added; none may be removed (`DocumentTypeUpdateError`, 40212). Each existing type follows the update rules of its keywords. |
| **Errors** | `DocumentTypesAreMissingError` (10214), `InvalidDocumentTypeNameError` (10415), at registration |

A contract has at least one document type, unless it defines tokens (`DocumentTypesAreMissingError`, 10214). A name is 1 to 64 ASCII letters, digits, `_` or `-`; from protocol version 14 a name may not contain `-` (`InvalidDocumentTypeNameError`, 10415). The keywords a schema takes are the subject of the rest of this part: see [Contract Keywords](../contract-keywords.md).

### `keywords` and `description`

| | |
|---|---|
| **Where** | contract |
| **Value** | `keywords`: array of at most 50 strings; `description`: string |
| **Default** | no keywords, no description |
| **Since** | protocol version 9 |
| **On update** | May be changed; the search entries are replaced |
| **Errors** | `TooManyKeywordsError` (10262), `InvalidKeywordLengthError` (10270), `InvalidKeywordCharacterError` (10269), `DuplicateKeywordsError` (10263), `InvalidDescriptionLengthError` (10264) |

The keyword search system contract indexes each contract by its keywords and description, so applications can find contracts by topic. The rules, checked on every create and update:

- At most 50 keywords (10262).
- Each keyword is 3 to 50 bytes of UTF-8 (10270) and contains no whitespace or control character (10269).
- No keyword appears twice (10263).
- A description is 3 to 100 bytes of UTF-8 (10264).

## `config`

| | |
|---|---|
| **Where** | contract |
| **Value** | object: `$formatVersion` and the keys below |
| **Default** | absent: every key takes its default |
| **Since** | protocol version 1 |
| **On update** | The keys below are fixed, with the exceptions each one names (`DataContractConfigUpdateError`, 40002) |
| **Errors** | `DataContractConfigUpdateError` (40002), `DataContractIsReadonlyError` (40001) |

`$formatVersion` is the config's own version: `"0"` before protocol version 9, `"1"` from 9, and `"2"` from 14, which adds `moderation`. From protocol version 14 every new contract carries config version 2, moderated or not, and an older contract moves to it with its next update.

### `canBeDeleted`

| | |
|---|---|
| **Where** | `config` |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 1 |
| **On update** | Fixed (40002) |

Whether the contract itself may ever be deleted. No transition deletes a contract today, so the flag has no effect yet beyond being recorded.

### `readonly`

| | |
|---|---|
| **Where** | `config` |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 1 |
| **On update** | Cannot be set by an update (40002) |
| **Errors** | `DataContractIsReadonlyError` (40001) |

`true` freezes the contract at its first version: every update of it is refused with `DataContractIsReadonlyError` (40001). Only a create can set it, so a contract is read-only from the start or never. A document reference can require its target contract to be read-only (`contractRequirements.readonly`, see [References](refers-to.md)).

### `keepsHistory`

| | |
|---|---|
| **Where** | `config` |
| **Value** | boolean |
| **Default** | `false` |
| **Since** | protocol version 1 |
| **On update** | Fixed (40002) |

`true` makes Drive keep every version of the contract, not only the latest, so a client can read and prove the contract as it was at an earlier version.

### Document type defaults

| | |
|---|---|
| **Where** | `config` |
| **Value** | `documentsKeepHistoryContractDefault`, `documentsMutableContractDefault`, `documentsCanBeDeletedContractDefault`: boolean each |
| **Default** | `false`, `true`, `true` |
| **Since** | protocol version 1 |
| **On update** | Fixed (40002) |

The value of `documentsKeepHistory`, `documentsMutable` and `canBeDeleted` for each document type that does not set it itself. A type that sets the keyword overrides the default. Changing a default would change every type that relies on it, so all three are fixed. See [History](history.md), [Mutability](mutability.md) and [Deletion](deletion.md).

### Bounded key requirements

| | |
|---|---|
| **Where** | `config` |
| **Value** | `requiresIdentityEncryptionBoundedKey`, `requiresIdentityDecryptionBoundedKey`: `0` unique, `1` multiple, `2` multiple with a pointer to the latest |
| **Default** | absent |
| **Since** | protocol version 1 |
| **On update** | Fixed (40002) |

Let identities add encryption, or decryption, keys bound to the whole contract, and say how they are kept: one key that cannot be replaced, several, or several with a pointer to the latest. The document type keywords of the same names do this for keys bound to one document type. See [Signing and Keys](signing-keys.md) and [Contract Bounds](../sdk/identity-keys.md#contract-bounds).

### `sizedIntegerTypes`

| | |
|---|---|
| **Where** | `config` |
| **Value** | boolean |
| **Default** | `true` in config version 1 and later; config version 0 has no such key and behaves as `false` |
| **Since** | protocol version 9 |
| **On update** | May be turned on, not off (40002) |

With `true`, each integer property is stored in the smallest width its `minimum`, `maximum` or `enum` allow: `"minimum": 0, "maximum": 100` takes one byte. With `false`, every integer is a signed 8-byte value. Turning it off would make stored documents unreadable, so it is refused. Turning it on is allowed by the config check, but it changes the width of every bounded integer of the existing document types, and an update that changes how an existing property's values are stored is refused (`DocumentTypeUpdateError`, 40212); in practice it can only be turned on when it leaves the width of every existing integer property unchanged.

The width also decides what a summed property may be: see [Counts, Sums and Averages](aggregates.md#documentssummable).

### `moderation`

| | |
|---|---|
| **Where** | `config` (config version 2) |
| **Value** | object: `banlist`, `suspensions`, `warnings` (booleans, default `false`) and `moderators` |
| **Default** | absent: the contract is not moderated |
| **Since** | protocol version 14 |
| **On update** | Which lists are kept is fixed, and an elected team can be neither declared, changed nor left; otherwise the moderators may change (40002) |
| **Errors** | `InvalidContractModerationConfigError` (10900), `ContractModeratorIdentityNotFoundError` (41110) |

Declares which moderation lists the contract keeps and who edits them. An identity on the banlist, or suspended, cannot act on the contract's documents; a warning is a record that bars nothing. `moderators` is one of:

- `{ "$type": "contractOwner" }`: the owner moderates alone.
- `{ "$type": "appointedModerators", "identities": [...] }`: the owner and 1 to 16 named identities, each acting alone. Every named identity must exist (`ContractModeratorIdentityNotFoundError`, 41110).
- `{ "$type": "elected", ... }`: a team elected by masternodes moderates, with the abilities the declaration gives it. See [Elected Moderation](../data-model/contract-moderation.md#elected-moderation).

A declaration keeps at least one list, unless a document type gives its moderators an ability (`moderatorAbilities`: deleting its documents or writing the fields it keeps for them), and is refused otherwise (`InvalidContractModerationConfigError`, 10900). An unknown key is refused rather than ignored, so a misspelled list name cannot silently leave the contract without it. Because the lists are fixed, a contract that will ever need moderation declares it when it is created.

`moderation` is what [`moderatorAbilities`](moderator-abilities.md) and the moderators' share of [`actionFees`](action-fees.md) require.

## See also

- [Data Contracts](../data-model/data-contracts.md) for the contract structure and its versions.
- [Contract Moderation](../data-model/contract-moderation.md) for the lists, the moderation transition and elected teams.
- [Contract Groups](../data-model/contract-groups.md) for contract groups, sets of contracts, which a create transition may register or join (not the contract's `groups`).
- [Contract Keywords](../contract-keywords.md) for the document type keywords.
