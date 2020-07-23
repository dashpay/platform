## Using a different account 

Because the Client uses mostly a mnemonic to initialize itself, you can access to the other account defined by the [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki).

As an helper for users and internal reference for `client.platform`. 
By default, accessing to `client.account` is equivalent of `client.wallet.getAccount({index:0})`. 
Therefore usage might varies if you need to deal with platform or not. 


### Access to account without platform
```js  
   const account = await client.getWalletAccount({index:1});
```

### Access to account with platform.

When calling `getWalletAccount`, the client will locally store the index options you have passed to it, which will be used for your platform related calls.

```js
async function changeAccount(){
   await client.getWalletAccount({index:2});
}
```
