exports.getLastBlock = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let lastHash = await self.Explorer.API.getLastBlockHash();
            let block = await self.Explorer.API.getBlock(lastHash);
            return resolve(block);
        });
    }
}