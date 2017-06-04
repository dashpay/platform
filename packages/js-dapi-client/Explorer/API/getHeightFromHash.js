const _fetch = require('../../util/fetcher.js')._fetch;
exports.getHeightFromHash = function(hash) {

    return new Promise(async function(resolve, reject) {
        SDK.Explorer.API.getBlock(hash)
            .then(function(_block) {
                resolve(_block.height);
            })
            .catch(err => reject(err));
    });
}