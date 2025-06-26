## Sign and verify messages

Dash SDK exports the Message constructor inside the Core namespace `new Dash.Core.Message`

```js
const Dash = require('dash');

const mnemonic = '';

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
