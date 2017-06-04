exports.getBlockMerkleRoot = function(identifier) {

    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getBlock(identifier)
            .then(function(_block) {
                resolve(_block.merkleroot);
            })
            .catch(err =>{
                reject(err);
            })
    });
}