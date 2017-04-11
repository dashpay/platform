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
- `SDK.Explorer.API.getLastBlockHeight()` - Retrieve the last height `Number`.
- `SDK.Explorer.API.getLastDifficulty()` - Retrieve the last diff `Number`.(float)
- `SDK.Explorer.API.getStatus()` - Retrieve information `Object`. (diff, blocks...)

##### Blockchain :

You will also able to use some internals. Mind that this is not advised.
##### EventEmitter :
- `SDK.emitter.emit()`

##### Socket :
- `SDK.socket.send()`

### Changelog
### License
