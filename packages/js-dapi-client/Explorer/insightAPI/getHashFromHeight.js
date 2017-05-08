const _fetch = require('../../util/fetcher.js')._fetch;
const axios = require('axios');

exports.getHashFromHeight = function() {
    let self = this;
    return async function(height){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/block-index/${height}`;
            return axios
                .get(url)
                .then(function(response){
                    if(response.hasOwnProperty('data') && response.data.hasOwnProperty('blockHash'))
                        return resolve(response.data.blockHash);
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