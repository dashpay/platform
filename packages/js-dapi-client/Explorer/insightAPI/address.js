const explorerGet = require('./common/ExplorerHelper').explorerGet;

exports.getBalance = function(addr) {

    return new Promise(function(resolve, reject) {
        explorerGet(`/addr/${addr}/balance`)
            .then(data => {
                resolve(data);
            })
            .catch(error => {
                reject(`An error was triggered while fetching address ${addr} :` + error);
            })
    });

}

exports.getUTXO = function(addr) {

    return new Promise(function(resolve, reject) {
        explorerGet(`/addr/${addr}/utxo`)
            .then(data => {
                resolve(data);
            })
            .catch(error => {
                reject(`An error was triggered while fetching address ${addr} :` + error);
            })
    });
}
