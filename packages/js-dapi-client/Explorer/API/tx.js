const explorerGet = require('./common/ExplorerHelper').explorerGet;
const axios = require('axios');

exports.send = function(rawtx) {

    return new Promise(function(resolve, reject) {

        let getInsightCandidate = SDK.Discover.getInsightCandidate();
        let getInsightURI = getInsightCandidate.URI;
        let url = `https://dev-test.dash.org/insight-api-dash/tx/send`;  //hard coded for now due to version issue
        // console.log('rawtx', rawtx)
        return axios //pvr: todo axios post abstraction
            .post(url, { rawtx: rawtx })
            .then(function(response) {
                return resolve(response.data);
            })
            .catch(function(error) {
                if (error) {
                    reject(error => { `An error was triggered while sending transaction: ` + error });
                }
            });
    });
}

exports.getTx = function(txId) {

    return explorerGet(`/tx/${txId}`)
        .then(data => {
            resolve(data);
        })
        .catch(error => {
            reject(`An error was triggered while getting transaction ${txId} by ID.} :` + error);
        })
}
