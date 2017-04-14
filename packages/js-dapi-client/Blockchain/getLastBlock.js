exports.getLastBlock = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let keys = Object.keys(self.Blockchain.blocks);
            keys.sort();
            let lastHeight = keys[keys.length-1];
            if(lastHeight){
                return resolve(self.Blockchain.blocks[lastHeight]);
            }else{
                return resolve(null);
            }
        });
    }
}