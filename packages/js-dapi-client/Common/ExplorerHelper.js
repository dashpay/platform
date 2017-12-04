const axios = require('axios')

function explorerPost(apiMethod, data) {
    return new Promise(function(resolve, reject) {

        axios.post(SDK.Discover.getConnectorCandidateURI() + apiMethod, data)
            .then(response => {
                resolve(response.data)
            })
            .catch(error => {
                reject(error);
            })
    })
};

function explorerGet(apiMethod) {
    return new Promise(function(resolve, reject) {
        axios.get(SDK.Discover.getConnectorCandidateURI() + apiMethod)
            .then(response => {
                resolve(response.data);
            })
            .catch(error => {
                reject(error);
            })
    });
}

module.exports = { explorerGet, explorerPost };
