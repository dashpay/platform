**Usage**: `sdk.platform.contracts.create(contractDefinitions, identity)`    
**Description**: This method will return a Contract object initialized with the parameters defined and apply to the used identity. 

Parameters: 

| parameters               | type              | required           | Description                                                       |  
|--------------------------|-------------------|------------------	| -----------------------------------------------------------------	|
| **contractDefinitions**  | JSONDataContract  | yes                | The defined [JSON Application Schema](https://dashplatform.readme.io/docs/explanation-platform-protocol-data-contract) |
| **identity**             | Identity          | yes                | A valid [registered `application` identity](/platform/identities/register.md) |

**Example**: 
```js
  const identityId = '';// Your identity identifier.
  const json = {};// Your valid json contract definitions
  const identity = await sdk.platform.identities.get(identityId);
  const contract = sdk.platform.contracts.create(json, identity);
```

Returns : Contract.
