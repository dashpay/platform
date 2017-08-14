const explorerGet = require('../../Common/ExplorerHelper').explorerGet;

exports.getHashFromHeight = function(height) {

    return new Promise(function(resolve, reject) {
        explorerGet(`/block-index/${height}`)
            .then(data => {
                resolve(data.blockHash);
            })
            .catch(error => {
                reject(`An error was triggered while fetching candidate  :` + error);
                //pvr: (`An error was triggered while fetching candidate ${getConnectorCandidate.idx} - signaling and removing from list`)
                //not sure why error message relates to insight candidate?
            })
    })
}
