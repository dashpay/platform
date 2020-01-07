## Use a local evonet

You can refer to https://github.com/dashevo/dash-network-deploy to deploy a devnet locally.   

You will then need to pass the seed ip, and [register the DPNS contract](https://github.com/dashevo/dpns-contract), and reference its `contractId` below.

```js
const seeds = [{service: '54.245.133.124'}];
const sdk = new DashJS.SDK({
  seeds,
  apps: {
    dpns: {
      contractId: '2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse'
    }
  }
});
```

After that, usage is the same.
