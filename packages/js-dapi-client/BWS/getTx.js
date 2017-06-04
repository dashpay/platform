const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;

exports.getTx=function(txid, cb){
    return new Promise(function(resolve, reject){
        explorerGet(`/tx/${txid}`)
            .then(resp=>{
                return resolve(cb(null, resp.data));
            })
            .catch(err => reject(err))
    });
};