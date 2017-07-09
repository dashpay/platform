const Node = require('./lib/node/node');
const config = require('./dapi.json');
const {isPortTaken} = require('./lib/utils');

let rep = config.node.rep;
let pub = config.node.pub;
let pubKey = config.node.pubKey;

async function prepareReplier() {
    let taken = await isPortTaken(rep.port);
    if(taken){
        rep.port++;
        await prepareReplier();
    }
    return true;
}
async function preparePublisher() {
    let taken = await isPortTaken(pub.port);
    if(taken){
        pub.port++;
        await preparePublisher();
    }
    return true;
}

async function starter(){
    await preparePublisher();
    await prepareReplier();
    try{
        let node = new Node({
            debug:true,
            rep:rep,
            pub:pub,
            pubKey:pubKey+rep.port//Just in order to make it unique. TO BE REMOVED TODO
        });    
    }catch (e) {
        console.log('Cannot start node...');
        console.error(e);
    }
}

starter();

process.on('uncaughtException', function (err) {
    console.log(err);
});