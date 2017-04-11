exports.getBlockBits = function() {
    let self = this;
    return async function(identifier){
        return new Promise(function (resolve, reject) {
            return self.Explorer.API.getBlock(identifier).then(function (_block) {
                return resolve(_block.bits);
            });
        });
    }
}