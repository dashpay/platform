const moment = require('moment');
const explorerGet = require('../Common/ExplorerHelper').explorerGet/*  */;
const lastHeight = require('../Explorer/API/getLastBlockHeight').getLastBlockHeight;

exports.getFeeLevels = function(network, cb) {
    return new Promise(function(resolve, reject) {
        explorerGet(`/utils/estimatefee`)
            .then(res => {
                //Pick the first value of the first key
                return res[Object.keys(res)[0]];
            })
            .then(fee => {
                if (cb) {
                    cb(null, fee);
                }
                return resolve(fee);
            })
            .catch(err => reject(err))
    });
};