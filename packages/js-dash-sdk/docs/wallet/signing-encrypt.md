## Sign a Transaction/Transition a message

```js
const tx = new DashJS.Transaction({
//txOpts
});
const signedTx = sdk.account.sign(tx);
```

## Encrypt a message

```js
  const message = 'Something';
  const signedMessage = sdk.account.encrypt('AES',message,'secret');
```

## Decrypt a message

```js
const encrypted = 'U2FsdGVkX19JLa+1UpbMcut1/QFWLMlKUS+iqz+7Wl4=';
const message = sdk.account.decrypt('AES',encrypted,'secret');
```
