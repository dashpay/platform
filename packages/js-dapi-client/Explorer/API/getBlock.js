const explorerGet = require('./common/ExplorerHelper').explorerGet;

exports.getBlock = function(identifier) {

    return new Promise(function(resolve, reject) {

        (Number.isInteger(identifier)
            ? SDK.Explorer.API.getHashFromHeight(identifier)
            : Promise.resolve(identifier))
            .then(id => {
                return explorerGet(`/block/${id}`)
            })
            .then(block => {
                resolve(block)
            })
            .catch(function(error) {
                //TODO : Signaling + removal feat
                console.log(error);
                reject("Unhandled error");
                // reject(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
            });
    });
}