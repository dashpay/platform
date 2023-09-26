## Sign and verify messages

Dash SDK exports the Message constructor inside the Core namespace `new Dash.Core.Message`

```js
const Dash = require('dash');

const mnemonic = '';

// Synchronization will take a lot of time with this configuration. 
// We can use `unsafeOptions.skipSynchronizationBeforeHeigth` option
// in combination with progress events as described here
// https://github.com/dashpay/platform/tree/v0.25-dev/packages/wallet-lib#usage
const client = new Dash.Client({
  wallet: {
    mnemonic,
  },
});

async function signAndVerify() {
  const account = await client.wallet.getAccount();

  const pk = new Dash.Core.PrivateKey();
  const message = new Dash.Core.Message('hello, world');
  const signed = account.sign(message, pk);
  const verified = message.verify(pk.toAddress().toString(), signed.toString());
}
```
