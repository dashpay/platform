const _fetch = require('../../util/fetcher.js')._fetch;
exports.getHeightFromHash = function() {
    let self = this;
    return async function(hash){
        return new Promise(async function (resolve, reject) {
            return self.Explorer.API.getBlock(hash).then(function (_block) {
                return resolve(_block.height);
            });
        });
    }
}