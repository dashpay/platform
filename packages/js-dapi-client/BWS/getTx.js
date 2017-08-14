const explorerGet = require('../Common/ExplorerHelper').explorerGet;

exports.getTx = function(txid) {
    return new Promise(function(resolve, reject) {
        explorerGet(`/tx/${txid}`)
            .then(resp => {
                return resolve(resp);
            })
            .catch(err => reject(err))
    });
};
