**Usage**: `client.platform.contracts.update(contract, identity)`
**Description**: This method will sign and broadcast an updated valid contract.

Parameters:

| parameters                | type      | required       | Description                                                                   |
|---------------------------|-----------|----------------| ------------------------------------------------------------------------------|
| **contract**              | Contract  | yes            | A valid [created contract](/platform/contracts/create.md)                     |
| **identity**              | Identity  | yes            | A valid [registered `application` identity](/platform/identities/register.md) |

**Example**:
You may check [following document](/examples/updating-a-contract.md) for an example on how to update a contract.

Returns : DataContractUpdateTransition.
