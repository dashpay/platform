/**
 * Created by maddogS on 2017-04-17.
 */
var ind = require('./index.js')
const DAPISDK = require('./index.js')//require('../dapi-sdk');
const options = {
    verbose:true,
    errors:true,
    warnings:true,
    debug:true,
    // DISCOVER:{
    //     INSIGHT_SEEDS:[{
    //         protocol:"https",
    //         path:'/api',
    //         base:"insight.dash.siampm.com",
    //         port: 80,
    //         fullPath:"http://192.168.0.20:3001/insight-api-dash"
    //     }]
    // }io
    DISCOVER:{
        INSIGHT_SEEDS:[

            {
                protocol:'https',
                path:"insight-api-dash",
                base:"dev-test.dash.org",
                port:443
                // https://dev-test.dash.org/insight-api-dash/block-headers/1000/10
            }
            //     {
            //     protocol:"https",
            //     path:'api',
            //     base:"insight.dash.siampm.com",
            //     port: 443
            // }
        ]
    }
};
const util = require('util');
global.SDK = {};
async function start(){
    global.SDK = await DAPISDK(options);
    let SDK = global.SDK;
    // await SDK.Blockchain.init({
    //     autoConnect:false,
    //     numberOfHeadersToFetch:25,
    //     fullFetch:true
    // });

    // setInterval(async function(){
    //     let height =  await SDK.Explorer.API.getLastBlockHeight()
    //     console.log('Height is', height);
    // },60000)
    // height="0000041461694567a06dccb44caebcd99b5075cbb0b5e96fdd0f1400aba1b483";
    var rootdata = {
        base: 'RootBase',
        params: '{BanMajority: 999, State: {Rating: -1}}',
        returns:  '{BanMajority,BanParticipation, State{Rating, Balance}}',
    }
    // let newRootObj = await SDK.Accounts.User.create(rootdata);
    // // console.log(61, newRootObj);
    //
    // accountKey = "test"
    //
    // var accountData={
    //     base: 'AccountBase',
    //     params: '{Action: 999, AccKey:'+'\"'+accountKey +'\"'+'}',
    //     returns:  '{Action,Type,AccKey,PubKey,Signature}',
    // }
    // let newAccObj = await SDK.Accounts.User.create(accountData);
    //
    // // console.log(72, newAccObj);
    //
    // var queryData={
    //     returns:  '{BanMajority,BanParticipation, State{Rating, Balance}}', //return default obj
    // }
    //
    // let queryRoot = await SDK.Accounts.User.search(queryData)
    // // console.log(79, queryRoot);

    // let addr = await SDK.Explorer.API.getBalance('yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4');
    // let UTXO= await SDK.Explorer.API.getUTXO('yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4');
    // let fee= await SDK.Explorer.API.estimateFees(4);
    let sent= await SDK.Explorer.API.send();
    let tx= await SDK.Explorer.API.getTx('dce49c9e92b1ba673dc7c2822c509c3850e09b4da766413bfd2051565ca1d396');



    // // console.log(82, addr)
    // console.log(83, UTXO)
    // console.log(84, fee)
    console.log(85, sent)
    console.log(86, tx)



    // let height2 = await SDK.Explorer.API.getHeightFromHash(hash);


    // let block = await SDK.Explorer.API.getBlock(height);
    // let block2 = await SDK.Explorer.API.getBlock(hash);
    //
    // let conf = await SDK.Explorer.API.getBlockConfirmations(hash);
    // let conf2 = await SDK.Explorer.API.getBlockConfirmations(height);
    // console.log(conf, conf2, conf.constructor.name)
    //
    // let size = await SDK.Explorer.API.getBlockSize(hash);
    // let size2 = await SDK.Explorer.API.getBlockSize(height);
    // console.log(size, size2, size.constructor.name)
    // console.log(block.size, block2.size, block.size.constructor.name);
    //
    // // console.log(util.inspect(SDK,{depth:10}))
    // let height = await SDK.Explorer.API.getLastBlockHeight();
    // let hash = await SDK.Explorer.API.getHashFromHeight(height);
    // console.log(hash);
    // let diff = await SDK.Explorer.API.getLastDifficulty();
    // console.log(height, diff);
    // console.log(diff.constructor.name);
    // console.log(`last height is ${height}`);
    // process.on('unhandledRejection', r => console.log(r));
    //

}
start();
