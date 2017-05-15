const axios = require('axios');

exports.getBalance = function(addr) {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/addr/${addr}/balance`;
            return axios
              .get(url)
              .then(function(response){
                console.log(url, response.data)
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    console.log(url, error)
                    console.error(`An error was triggered while fetching address ${addr} `);
                    return resolve(false);
                }
            });
        });
    }
}

exports.getUTXO = function(addr) {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;
            let url = `${getInsightURI}/addr/${addr}/utxo`;
            return axios
              .get(url)
              .then(function(response){
                return resolve(response.data);
              })
              .catch(function(error){
                if(error){
                    console.log(url, error)
                    console.error(`An error was triggered while fetching address ${addr} `);
                    return resolve(false);
                }
            });
        });
    }
}
