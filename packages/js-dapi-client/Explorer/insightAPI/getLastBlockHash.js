const _fetch = require('../../util/fetcher.js')._fetch;
exports.getLastBlockHash = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/status?q=getLastBlockHash`;
            _fetch({type: "GET", url: url}, function (err, data) {
                if(err){
                    console.error(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
                    //TODO: Do this thing!
                    return resolve(false);
                }
                return resolve(data.lastblockhash);
            });
        });
    }
}