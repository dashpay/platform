const explorerGet = require('../../Common/ExplorerHelper').explorerGet;
const explorerPost = require('../../Common/ExplorerHelper').explorerPost;
const axios = require('axios');

exports.send = function(rawtx) {
    return new Promise(function(resolve, reject) {
        return explorerPost(`/tx/send`, { rawtx })
            .then(data => {
                resolve(data);
            })
            .catch(error => {
                reject(`An error was triggered while sending tx ${rawtx}` + error);
            })
    });

}

exports.getTx = function(txId) {
    return new Promise(function(resolve, reject) {
        return explorerGet(`/tx/${txId}`)
            .then(data => {
                resolve(data);
            })
            .catch(error => {
                reject(`An error was triggered while getting transaction ${txId} by ID.} :` + error);
            })
    });

}
