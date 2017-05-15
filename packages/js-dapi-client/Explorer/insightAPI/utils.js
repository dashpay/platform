const axios = require('axios');

exports.estimateFees = function(blockNumber) {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/utils/estimatefee?nbBlocks=${blockNumber||2}`;
            return axios
              .get(url)
              .then(function(response){
                console.log(url, response.data)
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    console.log(url, error)
                    console.error(`An error was triggered getting fee estimates `);
                    return resolve(false);
                }
            });
        });
    }
}
