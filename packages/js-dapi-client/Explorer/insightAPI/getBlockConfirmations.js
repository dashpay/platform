exports.getBlockConfirmations = function(identifier) {

    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getBlock(identifier)
            .then(function(_block) {
                resolve(_block.confirmations);
            })
            .catch(error => reject(error));
    });
}