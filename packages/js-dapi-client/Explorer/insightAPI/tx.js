const axios = require('axios');

exports.send = function() {
    let self = this;
    return async function(rawtx){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `https://dev-test.dash.org/insight-api-dash/tx/send`;  //hard coded for now due to version issue
            console.log('rawtx', rawtx)
            return axios
              .post(url, {rawtx: rawtx})
              .then(function(response){
                // console.log("!!!", response.data)
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    // console.log("!!!", error.response.data)
                    console.log(url, error.response.data)
                    console.error(`An error was triggered while sending transaction.`);
                    return resolve(false);
                }
            });
        });
    }
}

exports.getTx = function() {
    let self = this;
    return async function(txId){
      console.log('txId', txId)
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/tx/${txId}`;
            console.log('url', url)
            return axios
              .get(url)
              .then(function(response){
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    console.log(url, error.response.data)
                    console.error(`An error was triggered while getting transaction ${txId} by ID.`);
                    return resolve(false);
                }
            });
        });
    }
}
