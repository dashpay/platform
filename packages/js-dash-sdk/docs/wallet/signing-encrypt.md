## Sign a Transaction/Transition a message

```js
const tx = new Dash.Transaction({
//txOpts
});
const signedTx = client.account.sign(tx);
```

## Encrypt a message

```js
  const message = 'Something';
  const signedMessage = client.account.encrypt('AES',message,'secret');
```

## Decrypt a message

```js
const encrypted = 'U2FsdGVkX19JLa+1UpbMcut1/QFWLMlKUS+iqz+7Wl4=';
const message = client.account.decrypt('AES',encrypted,'secret');
```
