// import blockchain parameters for Dash
const levelup = require('levelup');
const dashUtil = require('dash-util');
const base58 = require('base58');
global.SDK = require('../Connector/dapiFactory.js')();

// create a LevelUp database where the block data should be stored
var db = levelup('dash.chain', { db: require('memdown') })

// create blockchain
var Blockchain = require('blockchain-spv-dash')
var chain = new Blockchain(require('webcoin-dash-testnet').blockchain, db)

getTmpBlock = () => {

    let block1TestNet = `{"hash":"0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1","confirmations":254731,"size":186,"height":1,"version":2,"merkleroot":"b4fd581bc4bfe51a5a66d8b823bd6ee2b492f0ddc44cf7e820550714cedc117f","tx":["b4fd581bc4bfe51a5a66d8b823bd6ee2b492f0ddc44cf7e820550714cedc117f"],"time":1398712771,"mediantime":1398712771,"nonce":31475,"bits":"1e0fffff","difficulty":0.0002441371325370145,"chainwork":"0000000000000000000000000000000000000000000000000000000000200011","previousblockhash":"00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c","nextblockhash":"00000c6264fab4ba2d23990396f42a76aa4822f03cbc7634b79f4dfea36fccc2","isMainChain":true,"poolInfo":{"poolName":"Q/P2SH/"},"cbvalue":500}`
    let block1LiveNet = `{"hash": "000007d91d1254d60e2dd1ae580383070a4ddffa4c64c2eeb4a2f9ecc0414343", "confirmations": 724402, "size": 186, "height": 1, "version": 2, "merkleroot": "ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7", "tx": ["ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7"], "time": 1390103681, "mediantime": 1390103681, "nonce": 128987, "bits": "1e0ffff0", "difficulty": 0.000244140625, "chainwork": "0000000000000000000000000000000000000000000000000000000000200020", "previousblockhash": "00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6", "nextblockhash": "00000bafcc571ece7c5c436f887547ef41b574e10ef7cc6937873a74ef1efeae", "isMainChain": true, "poolInfo": { "poolName": "Q/P2SH/" }, "cbvalue": 500 }`

    return new Promise((resolve, reject) => {
        resolve(JSON.parse(block1TestNet))
    })
}

// wait for the blockchain to be ready
chain.on('ready', function() {

    // SDK.Explorer.API.getBlock(1)
    getTmpBlock()
        .then(block => {
            doDebugOutput(block)
            chain.addHeaders([SDK.Blockchain._normalizeHeader(block)], (err) => {
                if (err) console.log(err)
            })
        })
        .catch(ex => {
            console.log(ex)
        })
})

doDebugOutput = (block) => {
    // console.log(dashUtil.toHash('00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6'))
    console.log(dashUtil.toHash(block.previousblockhash));
    console.log(chain.getTip().hash);
}

