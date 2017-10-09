const axios = require('axios')

function explorerPost(apiMethod, data) {
    return new Promise(function(resolve, reject) {
        SDK.Discover.getConnectorCandidateURI()
            .then(uri => {
                uri += apiMethod;
                if (SDK._config.debug) console.log(`[EXPLORER][POST] ${uri}`);
                return axios.post(uri, data);
            })
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
        SDK.Discover.getConnectorCandidateURI()
            .then(uri => {
                uri += apiMethod;
                if (SDK._config.debug) console.log(`[EXPLORER][GET] ${uri}`);
                return axios.get(uri);
            })
            .then(response => {
                resolve(response.data);
            })
            .catch(error => {
                reject(error);
            })
    });
}

module.exports = { explorerGet, explorerPost };
