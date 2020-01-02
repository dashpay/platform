## About Contracts

At the core of each contracts lies a [JSON Application Schema](/getting-started/core-concepts#schemas).



## Create a new contract 

```js
const contract = sdk.platform.contracts.create(documentDefinitions, identity);
```   

## Registering a new contract 

```js
const contract = sdk.platform.contracts.create(documentDefinition,identity );
await sdk.platform.contracts.broadcast(contract, identity);
```   

## Fetching a new contract 

```js
const contract = sdk.platform.contracts.get('2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse');
```   
