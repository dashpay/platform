const explorerGet = require('../../Common/ExplorerHelper').explorerGet;

exports.getBlock = function(identifier) {

    return new Promise(function(resolve, reject) {
        let _id = null;

        Promise.resolve(Number.isInteger(identifier))
            .then(isInt => {
                return isInt ? SDK.Explorer.API.getHashFromHeight(identifier) : identifier
            })
            .then(id => {
                return explorerGet(`/block/${id}`)
            })
            .then(block => {
                resolve(block);
            })
            .catch(function(error) {
                reject(error);
            })
    })
}