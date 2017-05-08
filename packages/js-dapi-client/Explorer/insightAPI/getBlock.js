const _fetch = require('../../util/fetcher.js')._fetch;
const axios = require('axios');
exports.getBlock = function() {
    let self = this;
    return async function(identifier){
        return new Promise(async function (resolve, reject) {
            let hash = identifier;
            if(identifier.constructor.name=="Number"){hash = await self.Explorer.API.getHashFromHeight(identifier);}

            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/block/${hash}`;
            return axios
                .get(url)
                .then(function(response){
                    if(response.hasOwnProperty('data'))
                        return resolve(response.data);
                    else
                        return resolve(null);
                })
                .catch(function(error){
                    if(error){
                        //TODO : Signaling + removal feat
                        console.error(`An error was triggered while fetching candidate ${getInsightCandidate.idx} - signaling and removing from list`);
                        return resolve(false);
                    }
                });
        });
    }
}