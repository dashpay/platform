exports.getBlock = function() {
    let self = this;
    return async function(height){
        return new Promise(async function (resolve, reject) {
            if(!self.Blockchain.blocks.hasOwnProperty(height)) {
                let block = await self.Blockchain.blocks[height];
                return resolve(block);
            }
        });
    }
}