exports.getBlockVersion = function(identifier) {

    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getBlock(identifier)
            .then(function(_block) {
                return resolve(_block.version);
            })
            .catch(err => reject(err))
    });
}