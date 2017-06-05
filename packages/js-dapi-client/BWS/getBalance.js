const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;

exports.getBalance = function (twoStep, cb, addy) {
    return new Promise(function (resolve, reject) {
        SDK
            .Explorer
            .API
            .getBalance(addy)
            .then(function (res) {
                return resolve(res);
            })
            .catch(err => reject(err))
    });
};
