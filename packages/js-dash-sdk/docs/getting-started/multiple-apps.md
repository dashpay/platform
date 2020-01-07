# Working with multiple apps

When working with other registered contracts, you will need to know their `contractId` and reference it on the SDK constructor.

Assuming a contract DashPay and having a following `contractId: "2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse"`. 
You can then pass it as an options.

```js
const sdk = new DashJS.SDK({
  apps: {
    dashpay: {
      contractId: '2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse'
    }
  }
});
```

This allow the methods `sdk.platform.documents.fetch` to provide you field selection. 
Therefore, if the dashpay contract have a `profile` field that you wish to access, DashJS will allow you to do dot-syntax access :

```js
const bobProfile = await sdk.platform.documents.fetch('dashpay.profile', {name:'bob'})
``` 
