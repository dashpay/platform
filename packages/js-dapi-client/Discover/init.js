const has = require('../util/has.js');
const { uuid } = require('khal');

exports.init = function(query, update) {

    return new Promise(function(resolve, reject) {
        if (SDK._config.verbose) console.log('Discover - init - Fetch valid MN list');

        return SDK.Discover.Masternode.fetcher()
            .then(fetched => {
                if (!fetched || fetched.length == 0) {
                    reject('Explorer.API will throw an error if called as it has no INSIGHT-API seeds provided.');
                }
                else {
                    SDK.Discover.Masternode.validMNList = fetched
                }

                if (SDK._config.verbose) console.log(`Discover - init - Fetched ${fetched.length} MNs`);
                if (SDK._config.verbose) console.log(`Discover ready \n`)
                SDK.Discover._state = "ready";
                resolve(true);
            })
            .catch(err => {
                reject(err)
            })
    });
}