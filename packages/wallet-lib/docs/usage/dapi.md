## About DAPI

DAPI (Decentralized API) is a distributed and decentralized endpoints provided by the Masternode Network.  
You can learn more about DAPI on the [DAPI-Client documentation](https://dashevo.github.io/dapi-client/#/).

## Get the DAPI-Client instance

When the Wallet-lib is initialized without any transporter, Wallet-lib will by default use DAPI-Client as a transporter. 
You can fetch the current instance of DAPI directly from the wallet : 

```js
  const wallet = new Wallet();
  const client = wallet.transport;
```

## Modify the seeds

By using your own DAPI-Client instance and passing it to the Wallet constructor (using `transport` argument). You can specify your own seeds to connect to.  

```js 
const DAPIClient = require('@dashevo/dapi-client');
const { Wallet } = require('./src');
const DAPIClientTransport = require('./src/transport/DAPIClientTransport/DAPIClientTransport.js');

const client = new DAPIClient({
  seeds: [{ service: '18.236.131.253:3000' }],
  timeout: 20000,
  retries: 5,
});
const transport = new DAPIClientTransport(client);
const wallet = new Wallet({ transport });
```
