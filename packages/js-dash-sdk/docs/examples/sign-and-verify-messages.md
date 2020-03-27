## Sign and verify messages

Dash SDK exports the Message constructor inside the Core namespace `new Dash.Core.Message`.   

You can refer to its documentation : https://github.com/dashevo/dashcore-message/blob/master/README.md

```js
const pk = new Dash.Core.PrivateKey();
const message = new Dash.Core.Message('hello, world');
const signed = account.sign(message, pk);
const verify = message.verify(pk.toAddress().toString(), signed.toString());
```

See [code snippet](https://github.com/dashevo/DashJS/blob/master/examples/node/sign-and-verify-messages.js).
