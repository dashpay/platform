## About DAPI

DAPI (Decentralized API) is a distributed and decentralized endpoints provided by the Masternode Network.

## Get the DAPI-Client instance

When the Wallet-lib is initialized without any transporter, Wallet-lib will by default use DAPI-Client as a transporter. 
You can fetch the current instance of DAPI directly from the wallet : 

```js
  const wallet = new Wallet();
  const dapiInstance = wallet.transport;
```
