## Wallet

### Create a wallet

```
const { Wallet } = require('@dashevo/wallet-lib');
const { DAPIClient } = require('@dashevo/dapi-client');
const { Mnemonic } = require('@dashevo/dashcore-mnemonic');
const localForage = require('localforage');


const mnemonic = 'my mnemonic in 12 or 24 words;'; //Can also be an instance of Mnemonic
const network = 'testnet' // or 'livenet'
const transport = new DAPIClient();
const adapter = localForage.createInstance({
      name: 'persist:walletAdapter'
    });

const opts = {
    transport, mnemonic, network, adapter
};
const Wallet = new Wallet(opts);
```

## Create an Account

```
const opts = {
    mode: 'full',
    cacheTx: true
}
const account = wallet.createAccount(opts);
```

## Get an Account

```
const account = wallet.getAccount(0);
const account = wallet.getAccount(42, accountOpts); //If no account on index 42, will create with passed options
```


## Export a wallet

```
const hdPRivKey = wallet.exportWallet(true);
const mnemonic = wallet.exportWallet();
```

## Disconnect

```
wallet.disconnect();
```