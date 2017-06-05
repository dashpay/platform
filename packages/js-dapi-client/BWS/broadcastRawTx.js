const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;

exports.broadcastRawTx = function (opts, network, rawTx) {
    return new Promise(function (resolve, reject) {
        return SDK
            .Explorer
            .API
            .send(rawTx)
            .then(function (res) {
                console.log(res);
                return resolve(res);
            })
            .catch(function (err){
                console.log(err);
                reject(err)
            })
    });
};
