const DGW = require('dark-gravity-wave-js');
exports.expectNextDifficulty = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let lastBlock = self.Blockchain.getLastBlock();
            if(lastBlock && lastBlock.hasOwnProperty('height')){
                let lastHeight = lastBlock.height;
                let blockArr =[lastBlock];
                for(let i = lastHeight;i>(lastHeight-25);i--){
                    blockArr.push(self.Blockchain.getBlock(i));
                }
                if(blockArr.length==25){
                    blockArr = blockArr.map(function (_h) {
                        return {
                            height: _h.height,
                            target: `0x${_h.bits}`,
                            timestamp: _h.time
                        };
                    })
                    let nextbits = DGW.darkGravityWaveTargetWithBlocks(blockArr).toString(16);
                    return resolve(nextbits);
                }else{
                    return resolve(null);
                }
            }
            return resolve(null);
        });
    }
}