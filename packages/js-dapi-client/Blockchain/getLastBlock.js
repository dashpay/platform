exports.getLastBlock = function() {

    return new Promise(function(resolve, reject) {
        let keys = Object.keys(SDK.Blockchain.blocks);
        keys.sort();
        let lastHeight = keys[keys.length - 1];
        if (lastHeight) {
            resolve(SDK.Blockchain.blocks[lastHeight]);
        } else {
            reject(null);
        }
    }).then(lastHeight => {
        resolve(lastHeight);
    }).catch(err => {
        reject(err)
    })

}