## Using a different account 

Because the Client uses mostly a mnemonic to initialize itself, you can access to the other account defined by the [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki).

As an helper for users and internal reference for `client.platform`. 
By default, accessing to `client.account` is equivalent of `client.wallet.getAccount({index:0})`. 
Therefore usage might varies if you need to deal with platform or not. 


### Access to account without platform
```js  
   const accountIndex = 1;
   const account = client.wallet.getAccount({index:accountIndex});
   await account.isReady();
```

### Access to account with platform.

You will actually need to replace `client.account` to get platform to correctly fetch the right account to use for signing and fetching UTXOs.

```js
async function changeAccount(){
   client.account = client.wallet.getAccount({index:1});
   await client.account.isReady();
}
```
