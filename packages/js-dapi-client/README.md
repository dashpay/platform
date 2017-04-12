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
               protocol:"https",
               path:'/api',
               base:"insight.dash.siampm.com",
               port: 80,
               fullPath:"https://insight.dash.siampm.com/api"
           }]
       }
};
let SDK = await DAPISDK(options);
```

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

You will also able to use some internals. Mind that this is not advised.
##### EventEmitter :
- `SDK.emitter.emit()`

##### Socket :
- `SDK.socket.send()`


### Particular examples :

#### getBlockHeaders
```js
    let SDK = await DAPISDK();
    let height =  await SDK.Explorer.API.getLastBlockHeight();
    //This will fetch the last 25 blocks headers.
    let blockHeaders = await SDK.Explorer.API.getBlockHeaders(height, 25, -1);

    let hash="0000041461694567a06dccb44caebcd99b5075cbb0b5e96fdd0f1400aba1b483";//Hash for block 25
    //This will fetch from block 25 to block 124.
    let blockHeaders2 = await SDK.Explorer.API.getBlockHeaders(hash, 100, -1);

    //This will fetch from block 0 to block 24 (height:0, nb:25, direction:1)
    let blockHeaders3 = await SDK.Explorer.API.getBlockHeaders();
```

### Changelog
### License
