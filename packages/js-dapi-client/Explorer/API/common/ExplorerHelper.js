const axios = require('axios')

var exports = {};
exports.explorerGet = function(apiMethod) {

    return new Promise(function(resolve, reject) {
        SDK.Discover.getInsightCandidateURI()
            .then(uri => {
                return axios.get(`${uri}` + apiMethod);
            })
            .then(response => {
                resolve(response.data);
            })
            .catch(error => {
                reject(error);
            })
    });

}

module.exports = exports;