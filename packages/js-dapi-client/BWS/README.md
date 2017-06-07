
# BWS (v1 - june 7, 2017)

---
The purpose of this document is to show the list of BWS mock functions available, the params and examples.

## Notes: 
Some params will be placeholders for now (these are marked by ~)

* its important that you pass a param but what it is doesn't matter

Some routes will currently only returns static results . (these are marked by *#static*
)

## how to use example: 
BWS is promised based... you may consider using asyn await pattern.
...
```js
let broadcastResult = await SDK.BWS.broadcastRawTx(opts,network,rawTx);
```
...do stuff with broadcastResult.

## Transactions

**broadcastRawTx:** broadcasts a new transaction to the network. (Will not broadcast a already sent transaction -> will get a error.)

params: (opts~, network~, rawTx)

rawTX: a tx hash, 
> example: "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff13033911030e2f5032506f6f6c2d74444153482fffffffff0479e36542000000001976a914f0adf747fe902643c66eb6508305ba2e1564567a88ac40230e43000000001976a914f9ee3a27ef832846cf4ad40fe95351effe4a485d88acc73fa800000000004341047559d13c3f81b1fadbd8dd03e4b5a1c73b05e2b980e00d467aa9440b29c7de23664dde6428d75cafed22ae4f0d302e26c5c5a5dd4d3e1b796d7281bdc9430f35ac00000000000000002a6a283662876fa09d54098cc66c0a041667270a582b0ea19428ed975b5b5dfb3bca79000000000200000000000000"

returns: error or boolean


**getTx:** get tx details with a transaction id

params: (txid)

txid: a tx id, 
> example: "65d4f6369bf8a0785ae05052c86da4a57f76866805e3adadc82a13f7da41cbdf"

returns: Object with tx details 
> (see [insight API for example](http://insight.dev.dash.org/api/tx/02e7146fed1eeca237a0304d0d4252314773cc08273a37624bf4928275ccdd28))


**getTxHistory:** get transaction history of a address

params: (opts~, skip = 0, limit = 0, includeExtendedInfo=false)

skip: integer, i.e. skip the first 5 addresses (oldest 5)

limit: integer, i.e. only show a max of 20 address (returns only 20 results)

includeExtendedInfo: boolean, if **false** returns array of address, if **true** returns array of address objects. 

returns: 

array of address: ['yb21342iADyqAotjwcn4imqjvAcdYhnzeH', 'yUGETMg58sQd7mTHEZJKqaEYvvXc7udrsh']

 or 

if **includeExtendedInfo** is true returns a array of address object
> (see [insight API for example of address object](http://insight.dev.dash.org/api/addr/XfmtHzRb8TLGpE3z3bV9iMXr7N8UbNsLfk))


## Address

**getBalance:** returns the balance of the address

params: (twoStep~, cb~, address)

address: a adress, 
> example: yb21342iADyqAotjwcn4imqjvAcdYhnzeH

returns: balance (positive number)


**getMainAddress:** returns a list of address with valid balances for the HD wallet

params: (opts~, noVerify~, limit, reverse~, rootKey~, mnemonic, seed)

mnemonic: a _mnemonic with sufficient entropy (12 words)
> example: "handle pedal baby can instrument fish airport string stew design lick cable"

seed: a bip32 seed

limit: how many address max to return (address are generated in sequence)

returns: a list of addresses with none zero balance (but will return at least 1 even if balance is zero)


**getUtxos:** returns a list utxo for a given array of addresses

params: (opts~, addresses)

addresses: an array of addresses(string)
> example: ['yb21342iADyqAotjwcn4imqjvAcdYhnzeH', 'yUGETMg58sQd7mTHEZJKqaEYvvXc7udrsh']

returns: a list of sub arrays containing utxo of each address in the addresses array


## Utils

**getFeeLevels:** returns the current network fee level

params: (network*, cb-optional)

cb: a callback function, 
> example: x=>console.log(x)

returns: a  (positive number or -1 if error)


**getFiatRate:** returns a fiat rate. #static


(will eventually return a fiat rate based on ISO4217 currency code)

params: (network ~, cb-optional)

cb: a callback function, 
> example: x=>console.log(x)

returns: 
>Object: {ts: timestamp, rate: nuber (in fiat for 1 dash), fetchedOn: timestamp}




