## Updating an existing contract

To update your existing data contract you have to follow these steps:

### Fetch your exising contract

```js
const schema = {};// You JSON schema defining the app.
const client = new Dash.Client({
  wallet: {
    mnemonic: '', // Your app mnemonic, which holds the identity
  },
});

const exisingContractId = ''; // Your existing data contract id

const existingDataContract = await client.platform.contracts.get(exisingContractId);
```

### Update document definitions

```js
const documentDefinition = existingDataContract.getDocumentDefinition('myDocumentType');

// adding optional field
documentDefinition.properties.newField = {
  type: 'integer',
  minimum: 1,
};
```

### Update contract version

```js
existingDataContract.setVersion(existingDataContract.getVersion() + 1);
```

### Broadcast your changes

```js
await client.platform.contracts.update(existingDataContract, yourExistingIdentity);
```

**Note, that update will be only allowed if schema is backward compatible. Also, version incremented by 1 and only one of following fields updated: `$defs`, `documents` or `version`**
