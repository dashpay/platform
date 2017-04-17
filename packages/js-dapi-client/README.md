## DAPI-SDK

## Getting Started
### Using DAPI-SDK

Install DAPI-SDK from npm

```sh
npm i -S dapi-sdk
```

Install from github :

```sh
npm i -S github:Alex-Werner/dapi-sdk
```

Import the package :
```js
const DAPISDK = require('dapi-sdk');
```

Most of the SDK methods will returns you a Promise.
Therefore depending on your specific needs you can init or call methods of the SDK in a async way :
```js
DAPISDK()
    .then(function (SDK) {
        SDK
            .Explorer
            .API
            .getLastBlockHeight()
            .then(function (height) {
                console.log(`last height is ${height}`);
            })
    });
```

or using async/await features provided in last NodeJS version :
```js
let SDK = await DAPISDK();
let height = await SDK.Explorer.API.getLastBlockHeight();
console.log(`last height is ${height}`);
```

On the API documentation below, I will mostly using await.

#### Options
When initializing the SDK, you might want to pass some options.

- Verbose mode (log everything) : `verbose:true`. Default `false`
- Log errors : `errors:true`. Default `true`
- Log warnings : `warnings:true`. Default `true`
- Debug Mode : `debug:true`. Default `true`
- DISCOVER :
    - INSIGHT-SEEDS : DAPI-SDK will first try to fetch a list of insight-api from MN in seeds, you can also provide seeds by giving it here an `Array`

For instance, if you want DAPI-SDK to just shut the fuck up :
```js
const options = {
    debug:false,
    verbose:false,
    errors:false,
    warnings:false
};
let SDK = await DAPISDK(options);
```
Or if you want DAPI-SDK to have specific insight api as seeds :

```js
const options = {
   DISCOVER:{
           INSIGHT_SEEDS:[{
               //fullPath: https://insight.dash.siampm.com/api
               protocol:"https",
               path:'api',
               base:"insight.dash.siampm.com",
               port: 443
           }]
       }
};
let SDK = await DAPISDK(options);
```


You will then have access to many components :

- `Explorer` will allow you to perform some command on the Insight-API of a masternode (chosen randomnly on a validated list).
As it will use some `Discover` methods in order to get the Insight Candidate, calling an Explorer call will first perform an init of Discover (and therefore will fetch and validate the list) before returning the value.
Make a note that this will be performed only once.

- `Blockchain` will allow you to have access to some components such as : getting an object stored in your in-mem db, or validate multiple blocks, calculate the next difficulty.
The initialization must be done by yourself using : `await SDK.Blockchain.init()` in order to beneficiate theses function.
Make note that by default it will connect to a randomnly selected insight-api by websocket and will listen to all block. When a block is emitter by the API, it will add it in the blockchain.
This comportement can be disable by passing to init the corresponding options (see below).
Using `SDK.Blockchain.chain` enable you to use the [blockchain-spv-dash](https://github.com/snogcel/blockchain-spv-dash) methods

- `tools` will allow to access to some of the dependencies of the SDK. Most notably, you have access to :
    - `SDK.tools.util` which correspond to a library of handy stuff such as toHash(hex), compressTarget, expandTarget. API here : [dash-util](https://github.com/snogcel/dash-util)
    - `SDK.tools.bitcore` which correspond to a library used in insight-api, see API here : [bitcore-dash-library](https://github.com/dashpay/bitcore-lib-dash). Contains elements that allow to generate address, sign/verify a message...

### API

##### Accounts :
WIP STATE
- `SDK.Accounts.User.create()`
- `SDK.Accounts.User.login()`
- `SDK.Accounts.User.remove()`
- `SDK.Accounts.User.search()`
- `SDK.Accounts.User.send()`
- `SDK.Accounts.User.update()`

##### Wallet :
TBA
##### Discover :
- `SDK.Discover.getInsightCandidate()` - Get a random insightAPI object (URI);

##### Explorer :
###### RPC :
TBA
###### InsightAPI :
- `SDK.Explorer.API.getStatus()` - Retrieve information `Object`. (diff, blocks...)
- `SDK.Explorer.API.getBlock(hash|height)` - Retrieve block information `Object` from either an hash `String` or an height `Number`
   It worth mentioning that retrieving from height is slower (2 call) than from an hash you might want to use Blockchain method instead.
- `SDK.Explorer.API.getLastBlockHash(hash)` - Retrieve last block hash `String`.
- `SDK.Explorer.API.getHashFromHeight(height)` - Retrieve hash value `String` from an height `Number|String`.
- `SDK.Explorer.API.getBlockHeaders(hash|height, [nbBlocks,[direction]])` - Retrieve 25 or `Number` of block headers `Array[Object]` from an height `Number` or a Hash `String` in a `Number` direction (see exemple below).
    This feature is not propagated everywhere yet. It has been pushed some weeks ago but even our official insight api do not reflect it - yet.

In this section, you retrieve a single value from one of the call (above), see theses methods as aliases.
- `SDK.Explorer.API.getLastBlockHeight()` - Retrieve the last height `Number`.
- `SDK.Explorer.API.getLastDifficulty()` - Retrieve the last diff `Number`.(float)
- `SDK.Explorer.API.getHeightFromHash(hash)` - Retrieve hash value `Number` from an hash `String`.
- `SDK.Explorer.API.getBlockConfirmations(hash|height)` - Retrieve the `Number` of confirmations of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockSize(hash|height)` - Retrieve the size `Number` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockBits(hash|height)` - Retrieve the bits `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockChainwork(hash|height)` - Retrieve the chainwork `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockMerkleRoot(hash|height)` - Retrieve the merkle root `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockTransactions(hash|height)` - Retrieve the transactions `Array[String]` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockTime(hash|height)` - Retrieve the timestamp (epoch in sec) `Number` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockVersion(hash|height)` - Retrieve the version `Number` of any block height `Number` or block hash `String`.

##### Blockchain :
DAPI-SDK has a internal Blockchain. These function will use the internal blockchain when possible and will retrieve when it won't.

- `SDK.Blockchain.init([options])` - Initialize the blockchain in order to be used. Optional default can be changed by passing one of these options :
    - options :
        - `autoConnect` - `Boolean` by default `true`. Disabling it will prevent the automatic socket connection.
        - `numberOfHeadersToFetch` - `Number` by default `100`, allow to specify how many headers to fetch at init.
        - `fullFetch` - `Boolean` by default `false`. Activating it allow to fetch all the blockchain headers from genesis to last tip. (event `fullFetched` emitted when end)
        This way you can setup a full blockchain fetch (numberOfHeadersFetched will then be ignored).

//- `SDK.Blockchain.expectNextDifficulty()` - Will expect the likely difficulty `Number` of the next block.
//- `SDK.Blockchain.validateBlocks(hash|height, [nbBlocks,[direction]])` - Will validate 25 or `Number` of block headers from an height `Number` or a Hash `String` in a `Number` direction.
//- `SDK.Blockchain.getBlock(height)` - Will return a block by it's height `Number`.
//- `SDK.Blockchain.getLastBlock()` - Will return the last block stored.
//- `SDK.Blockchain.addBlock(block)` - Will add a block headers.

The blockchain provide you also some events such as
    - `ready`
    - `socket.connected` - Where the argument provided is the socket itself.
    - `socket.block` - Where the argument provided is the block.
    - `socket.tx` - Where the argument provided is the TX.


### Internals

You will also able to use some internals. Mind that this is not advised.
##### EventEmitter :
//- `SDK.emitter.emit()`
##### Socket :
//- `SDK.socket.send()`
### Particular examples and usecases :

You can see some usecases and examples : [here](USECASES.md)

### [Changelog](CHANGELOG.md)
### License
