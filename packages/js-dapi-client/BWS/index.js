exports.BWS = function(){
    return {
        broadcastRawTx:require('./broadcastRawTx').broadcastRawTx,
        getBalance:require('./getBalance').getBalance,
        getFeeLevels:require('./getFeeLevels').getFeeLevels,
        getFiatRate:require('./getFiatRate').getFiatRate,
        getMainAddress:require('./getMainAddress').getMainAddress,
        getTx:require('./getTx').getTx,
        getTxHistory:require('./getTxHistory').getTxHistory,
        getUtxos:require('./getUtxos').getUtxos,
    }    
};