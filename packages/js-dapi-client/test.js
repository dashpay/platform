/**
 * Created by maddogS on 2017-04-17.
 */
var ind = require('./index.js')
const DAPISDK = require('./index.js')//require('../dapi-sdk');
const options = {
    verbose: true,
    errors: true,
    warnings: true,
    debug: true,
    DISCOVER: {
        INSIGHT_SEEDS: [
            {
                protocol: 'http',
                path: "/insight-api-dash",
                base: "51.15.5.18",
                port: 3001
            },
            {
                protocol: 'https',
                path: "/insight-api-dash",
                base: "dev-test.dash.org",
                port: 443
            }
        ]
    }
};
const util = require('util');
global.SDK = {};
async function start() {
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
        returns: '{BanMajority,BanParticipation, State{Rating, Balance}}',
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
    // let sent= await SDK.Explorer.API.send();
    // let tx= await SDK.Explorer.API.getTx('dce49c9e92b1ba673dc7c2822c509c3850e09b4da766413bfd2051565ca1d396');

    // ===== bws ====>

    // let bwsfee = await SDK.BWS.BWS.getFeeLevels()('live',(err, x)=>{console.log('bws fee', x)})
    let bwsutxo = await SDK.BWS.getUtxos('nada', ['yb21342iADyqAotjwcn4imqjvAcdYhnzeH', 'yUGETMg58sQd7mTHEZJKqaEYvvXc7udrsh'])
    // let bwstx = await SDK.BWS.BWS.getTx()('65d4f6369bf8a0785ae05052c86da4a57f76866805e3adadc82a13f7da41cbdf',(err, x)=>{console.log('bws tx', x)})
    // let bwsbal = await SDK.BWS.BWS.getBalance()('yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4',(err, x)=>{console.log('bws balance', x)})
    // let bwssend= await SDK.BWS.broadcastRawTx(1,1,'01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff13033911030e2f5032506f6f6c2d74444153482fffffffff0479e36542000000001976a914f0adf747fe902643c66eb6508305ba2e1564567a88ac40230e43000000001976a914f9ee3a27ef832846cf4ad40fe95351effe4a485d88acc73fa800000000004341047559d13c3f81b1fadbd8dd03e4b5a1c73b05e2b980e00d467aa9440b29c7de23664dde6428d75cafed22ae4f0d302e26c5c5a5dd4d3e1b796d7281bdc9430f35ac00000000000000002a6a283662876fa09d54098cc66c0a041667270a582b0ea19428ed975b5b5dfb3bca79000000000200000000000000',(err, x)=>{console.log('bws balance', x)}); //other params
    // let bwstxhist = await SDK.BWS.BWS.getTxHistory()({}, 0, 10, true, (err, x)=>{console.log('bws txhist', x)})
    // let bwsFiatRate= await SDK.BWS.BWS.getFiatRate()({},{},{},{}, (err, x)=>{console.log('bws fiat rate', x)}); //other params

    // let bwsAddy = await SDK.BWS.BWS.getMainAddress()({},{},10,{},(err, x)=>{console.log('bws mainaddy', x)},{},'inflict about smart zoo ethics ignore retire expand peasant draft sock raven')

    // console.log(82, addr)
    // console.log(83, bwsutxo)
    // console.log(84, bwsfee)
    // // console.log(85, sent)
    // console.log(86, bwstx)
    // // console.log(83, fee)
    // console.log(85, bwsbal)
    // console.log(88, bwssend)
    // console.log(89, bwstxhist)
    // console.log(93, bwsAddy)








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
