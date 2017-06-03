exports.getLastBlock = function() {

    return new Promise(function(resolve, reject) {
        SDK.Explorer.API.getLastBlockHash()
            .then(lastHash => {
                return SDK.Explorer.API.getBlock(lastHash)
            })
            .then(block => resolve(block))
    });
}