exports.addBlock = function() {
    let self = this;
    return async function(_block){
        return new Promise(async function (resolve, reject) {
            if(!self.Blockchain.blocks.hasOwnProperty(_block.height)){
                self.Blockchain.blocks[_block.height]=_block;
            }
            return resolve(true);
        });
    }
}