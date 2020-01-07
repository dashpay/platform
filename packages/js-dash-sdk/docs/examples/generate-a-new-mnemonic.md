## Generate a new mnemonic

In order to be able to keep your private keys private, we encourage to create your own mnemonic instead of using those from the examples (that might be empty).
Below, you will be proposed two options allowing you to create a new mnemonic, depending on the level of customisation you need. 

## DashJS.SDK

By passing `null` to the mnemonic value, you can get Wallet-lib to generate a new mnemonic for you. 

```js
const DashJS = require("dash");
const sdk = new DashJS.SDK({
  network: "testnet",
  mnemonic: null,
});
const mnemonic = sdk.wallet.exportWallet();
console.log({mnemonic});
```

## DashJS.Mnemonic 

```js
const DashJS = require("dash");
const {Mnemonic} = DashJS;

const mnemnonic = new Mnemonic.toString()
```

### Language selection 

```js
const {Mnemonic} = DashJS;
const {CHINESE, ENGLISH, FRENCH, ITALIAN, JAPANESE, SPANISH} = Mnemonic.Words;
console.log(new Mnemonic(Mnemonic.Words.FRENCH).toString())
```

### Entropy size

By default, the value for mnemonic is `128` (12 words), but you can generate a 24 words (or other) : 

```js
const {Mnemonic} = DashJS;
console.log(new Mnemonic(256).toString())
```

You can even replace the word list by your own, providing a list of 2048 unique words.
