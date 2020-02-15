**Usage**: `async client.getUTXO(address, from, to, fromHeight, toHeight)`
**Description**: Returns UTXO for a given address or multiple addresses (max result 1000)

Parameters:

| parameters             | type               | required       | Description                                                                                             |
|------------------------|--------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **address**            | String/[String]    | yes            | address or array of addresses |
| **from**               | Number             | no             | start of range in the ordered list of latest UTXO |
| **to**                 | Number             | no             | end of range in the ordered list of latest UTXO |
| **fromHeight**         | Number             | no             | which height to start from (optional, overriding from/to) |
| **toHeight**           | Number             | no             | on which height to end (optional, overriding from/to) |

Returns : Promise<object> - Object with pagination info and array of unspent outputs

