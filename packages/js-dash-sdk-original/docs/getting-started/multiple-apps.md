# Working with multiple apps

When working with other registered contracts, you will need to know their `contractId` and reference it in the SDK constructor.

Assuming a contract DashPay has the following `contractId: "77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3"`. 
You can then pass it as an option.

```js
const client = new Dash.Client({
  apps: {
    dashpay: {
      contractId: '77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3'
    }
  }
});
```

This allow the method `client.platform.documents.get` to provide you field selection. 
Therefore, if the contract has a `profile` field that you wish to access, the SDK will allow you to use dot-syntax for access :

```js
const bobProfile = await client.platform.documents.get('dashpay.profile', { name: 'bob' });
``` 
