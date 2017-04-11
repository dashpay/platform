const _fetch = require('../../util/fetcher.js')._fetch;
exports.getBlock = function() {
    let self = this;
    return async function(identifier){
        return new Promise(async function (resolve, reject) {
            let hash = identifier;
            if(identifier.constructor.name=="Number"){hash = await self.Explorer.API.getHashFromHeight(identifier);}

            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/block/${hash}`;
            _fetch({type: "GET", url: url}, function (err, data) {
                if(err){
                    console.error(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
                    //TODO: Do this thing!
                    return resolve(false);
                }
                if (data) {
                    return resolve(data);
                } else {
                    return resolve(null);
                }
            });
        });
    }
}