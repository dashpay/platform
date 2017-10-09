'use strict'

var fs = require('fs');
const utils = require('../lib/utils');
const DashUtil = require('dash-util');
const config = require('../config/config')


let getNewBlock = function(prev, bits) {
    return utils.createBlock(prev, parseInt(bits, 16));
}

var generateHeaders = function() {

    var blocks = [];

    //chain 1 block 1 - connects to genesis
    blocks.push(getNewBlock(config.getLowDiffGenesis(), '1fffffff')); //0

    //chain 2 block 1 - connects to genesis
    blocks.push(getNewBlock(config.getLowDiffGenesis(), '1fffff0d')); //1

    //chain 2 block 2
    blocks.push(getNewBlock(blocks[1], '1fffff0c')); //2

    //chain 1 block 2
    blocks.push(getNewBlock(blocks[0], '1ffffffd')); //3

    //chain 2 block 3 - first matured block & cumalative difficulty higher than chain 1 
    //thus the first block considered main chain
    blocks.push(getNewBlock(blocks[2], '1fffff0b')); //4

    return blocks;
}


var getMerkleTestHeaders = function(startHeight) {

    //When available....
    //SDK.Explorer.API.getHeaders(startHeight) //Get max headers --> returns a promise

    return new Promise((resolve, reject) => {


        let blocksTestNet =
            [
                { "version": 2, "merkleroot": "b4fd581bc4bfe51a5a66d8b823bd6ee2b492f0ddc44cf7e820550714cedc117f", "time": 1398712771, "nonce": 31475, "bits": "1e0fffff", "previousblockhash": "00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c" },
                { "version": 2, "merkleroot": "0d6d332e68eb8ecc66a5baaa95dc4b10c0b32841aed57dc99a5ae0b2f9e4294d", "time": 1398712772, "nonce": 6523, "bits": "1e0ffff0", "previousblockhash": "0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1" },
                { "version": 2, "merkleroot": "1cc711129405a328c58d1948e748c3b8f3d610e66d9901db88c42c5247829658", "time": 1398712774, "nonce": 53194, "bits": "1e0ffff0", "previousblockhash": "00000c6264fab4ba2d23990396f42a76aa4822f03cbc7634b79f4dfea36fccc2" },
            ]

        let block1LiveNet = { "hash": "000007d91d1254d60e2dd1ae580383070a4ddffa4c64c2eeb4a2f9ecc0414343", "confirmations": 724402, "size": 186, "height": 1, "version": 2, "merkleroot": "ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7", "tx": ["ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7"], "time": 1390103681, "mediantime": 1390103681, "nonce": 128987, "bits": "1e0ffff0", "difficulty": 0.000244140625, "chainwork": "0000000000000000000000000000000000000000000000000000000000200020", "previousblockhash": "00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6", "nextblockhash": "00000bafcc571ece7c5c436f887547ef41b574e10ef7cc6937873a74ef1efeae", "isMainChain": true, "poolInfo": { "poolName": "Q/P2SH/" }, "cbvalue": 500 }
        resolve(blocksTestNet.map(b => utils._normalizeHeader(b)));
    });
}

module.exports = {
    fetchHeaders: function(path = '/data/testchain.dat') {
        return generateHeaders();
    }
}