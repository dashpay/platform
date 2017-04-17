# USE CASES

The purpose of this document is to provide some use-cases and exemple on how you can use this SDK.


### getBlockHeaders
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


### Usage of Blockchain and events.
```js
    const options = {
        verbose:true,
        errors:true,
        warnings:true,
        debug:true,
        DISCOVER:{
            INSIGHT_SEEDS:[
                {
                    protocol:'https',
                    path:"insight-api-dash",
                    base:"dev-test.dash.org",
                    port:443
                }
            ]
        }
    };
    let SDK = await DAPISDK(options);
    SDK.Blockchain.init({
        autoConnect:true,
        numberOfHeadersToFetch:25,
        fullFetch:true
    });
    SDK.Blockchain.emitter.on('ready',function(){
        console.log('ready');
    });
    SDK.Blockchain.emitter.on('socket.connected',function(){
        console.log('socket connected');
    });
    SDK.Blockchain.emitter.on('socket.block',function(){
       console.log('block received');
    });
    SDK.Blockchain.emitter.on('socket.tx',function(){
        console.log('tx received');
    });
```