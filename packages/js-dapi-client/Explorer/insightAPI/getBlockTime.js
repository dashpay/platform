
exports.getBlockTime = function(identifier) {

    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getBlock(identifier)
            .then(function(_block) {
                resolve(_block.time);
            })
            .catch(err => reject(err))
    });
}