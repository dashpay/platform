const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;

exports.getFiatRate = function (opts, ccyCode, ts, provider, cb) {
    return new Promise(function (resolve, reject) {
        return resolve(cb(null,{ts: Date.now() - 3000, rate: 120, fetchedOn: Date.now()}))
    });
};