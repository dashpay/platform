const _fetch = require('../../util/fetcher.js')._fetch;
const axios = require('axios');
const explorerGet = require('./common/ExplorerHelper').explorerGet;

exports.getLastBlockHash = function() {

    return new Promise(function(resolve, reject) {
        explorerGet(`/status?q=getLastBlockHash`)
            .then(data => {
                if (data.hasOwnProperty('lastblockhash'))
                    resolve(data.lastblockhash);
                else
                    reject(null);
            })
            .catch(error => {
                reject(`An error was triggered while fetching address ${addr} :` + error);
            })
    });
}