const axios = require('axios');

exports.estimateFees = function() {
    let self = this;
    return async function(blockNumber) {
        return new Promise(async function(resolve, reject) {
            // let getConnectorCandidate = await self.Discover.getConnectorCandidate(); todo
            let getInsightURI = getConnectorCandidate.URI;
            let url = `${getInsightURI}/utils/estimatefee?nbBlocks=${blockNumber || 2}`;
            return axios
                .get(url)
                .then(function(response) {
                    console.log(url, response.data)
                    return resolve(response.data);
                })
                .catch(function(error) {
                    if (error) {
                        console.log(url, error)
                        console.error(`An error was triggered getting fee estimates `);
                        return resolve(false);
                    }
                });
        });
    }
}
