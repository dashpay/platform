const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;

exports.getTxHistory = function (opts, skip = 0, limit = 0, includeExtendedInfo) {
    return new Promise(function (resolve, reject) {
        let promises = [];

        function fetchingTxHistoryWithExtendedInfo(response) {
            response.transactions.forEach(txId => {
                promises.push(explorerGet(`/tx/${txId}`));
            });
            return Promise
                .all(promises)
                .then(res => {
                    return resolve(res);
                })
              
        };
        if (!opts.hasOwnProperty('addr')) {
            return reject('Missing param addr in opts');
        }
        return explorerGet(`/addr/${opts.addr}?from=${skip}&to=${limit}`)
            .then(function (response) {
                return includeExtendedInfo ? fetchingTxHistoryWithExtendedInfo(response) : resolve(response.transactions);
            })
            .catch(function (error) {
                if (error) {
                    return reject(`An error was triggered getting getTxHistory` + error);
                }
            });
    });
};
