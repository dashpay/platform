exports.getLastBlockHeight = function() {

    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getStatus()
            .then(function(_status) {
                resolve(_status.info.blocks);
            })
            .catch(err => reject(err))
    });
}