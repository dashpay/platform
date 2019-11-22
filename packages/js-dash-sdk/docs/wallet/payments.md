## Pay to an address

```js
const options = {
    recipient: "yizmJb63ygipuJaRgYtpWCV2erQodmaZt8",
    satoshis:100000,
    isInstantSend:false
};
const transaction = account.createTransaction(options);
const txid = account.broadcastTransaction(transaction);
```
