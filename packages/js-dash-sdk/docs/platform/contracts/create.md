**Usage**: `client.platform.contracts.create(contractDefinitions, identity)`    
**Description**: This method will return a Contract object initialized with the parameters defined and apply to the used identity. 

Parameters: 

| parameters               | type              | required           | Description                                                       |  
|--------------------------|-------------------|------------------	| -----------------------------------------------------------------	|
| **contractDefinitions**  | JSONDataContract  | yes                | The defined [JSON Application Schema](https://dashplatform.readme.io/docs/explanation-platform-protocol-data-contract) |
| **identity**             | Identity          | yes                | A valid [registered `application` identity](/platform/identities/register.md) |

**Example**:

```js
  const identityId = '';// Your identity identifier.
  
  // Your valid json contract definitions
  const contractDefinitions = {
    note: {
      properties: {
        message: {
          type: "string"
        }
      },
      additionalProperties: false
    }
  };
  const identity = await client.platform.identities.get(identityId);
  const contract = client.platform.contracts.create(contractDefinitions, identity);
  
  // You can use the validate method from DPP to validate the created contract
  const validationResult = client.platform.dpp.dataContract.validate(contract);
```

**Note**: When your contract is created, it will only exist locally, use the [broadcast](/platform/contracts/broadcast.md) method to register it.  

Returns : Contract.
