## Obtain account
```js
const account = await client.wallet.getAccount();
```

## Sign a Transaction


```js
const tx = new Dash.Core.Transaction({
  // ...txOpts
});
const signedTx = account.sign(tx);
```

## Encrypt a message

```js
  const message = 'Something';
  const signedMessage = account.encrypt('AES', message, 'secret');
```

## Decrypt a message

```js
const encrypted = 'U2FsdGVkX19JLa+1UpbMcut1/QFWLMlKUS+iqz+7Wl4=';
const message = account.decrypt('AES', encrypted, 'secret');
```
