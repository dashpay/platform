const DGW = require('dark-gravity-wave-js');
exports.expectNextDifficulty = function() {
    let self = this;
    return async function() {
        return new Promise(async function(resolve, reject) {
            let lastBlock = await self.Blockchain.getLastBlock();
            console.log("Last", lastBlock.hash);
            if (lastBlock && lastBlock.hasOwnProperty('height')) {
                let lastHeight = lastBlock.height;
                console.log("hgieht", lastHeight);
                let blockArr = [lastBlock];
                for (let i = lastHeight; i > (lastHeight - 24); i--) {
                    let block = await self.Blockchain.getBlock(i)
                    if (block) {
                        blockArr.push(block);
                    }
                    else {
                        return resolve(null);
                    }
                }
                console.log(blockArr.length);
                if (blockArr.length == 25) {
                    blockArr = blockArr.map(function(_h) {
                        return {
                            height: _h.height,
                            target: `0x${_h.bits}`,
                            timestamp: _h.time
                        };
                    })
                    let nextbits = DGW.darkGravityWaveTargetWithBlocks(blockArr).toString(16);
                    return resolve(nextbits);
                } else {
                    return resolve(null);
                }
            }
            return resolve(null);
        });
    }
}