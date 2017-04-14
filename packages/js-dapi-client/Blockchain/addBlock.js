const {clone} = require('khal');
const Buffer = require('../util/Buffer');
const DashUtil = require('dash-util')
const Bitcore = require('bitcore-lib-dash');

exports.addBlock = function() {
    let self = this;
    return async function(_block){
        return new Promise(async function (resolve, reject) {
            if(!self.Blockchain || !self.Blockchain.hasOwnProperty('isChainReady') || !self.Blockchain.isChainReady===true){
                //Then delay the addBlock for a retest later
                //TODO : Might be better to add this in a queue that would be handle when the ready event is thrown (we then handle the Q one by one)
            }
            const normalizeBlock = function(_b){
                let _el = clone(_b);

                let bh = {
                    version:_el.version,
                    prevHash:DashUtil.toHash(_el.previousblockhash),
                    merkleRoot:DashUtil.toHash(_el.merkleroot),
                    time:_el.time,
                    bits:parseInt(_el.bits,16),
                    nonce:_el.nonce
                };
                console.log(bh);
                return new Bitcore.BlockHeader.fromObject(bh);

            };

            let block = normalizeBlock(_block);
            console.log(block);

            let chain = self.Blockchain.chain;
            // let hash = _block.hash;
            // console.log(hash)
            // block.hash = Buffer.from(block.hash,'utf8');
            // block.merkleroot = u.toHash(block.merkleroot);
            // block.previousblockhash
            // console.log(Buffer.isBuffer(block.hash));
            // console.log(block);
            let array = [];
            array.push(block);
            chain.addHeaders(array, function(error, success){
                console.log(error);
                console.log(success);
                var tip = chain.getTip();
                console.log('synced to: ' + tip.height);

            });
            // if(!self.Blockchain.blocks.hasOwnProperty(_block.height)){
            //     self.Blockchain.blocks[_block.height]=_block;
            // }
            return resolve(true);
        });
    }
}