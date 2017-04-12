const has = require('../util/has.js');
const {uuid}=require('khal');

exports.init = function() {
    let self = this;
    return async function(query, update){
        return new Promise(async function (resolve, reject) {
            if(self._config.verbose) console.log('Blockchain - init - try to restore Blockchain state');
            let restored = await self.Blockchain.restore();
            if(self._config.verbose) console.log(`Blockchain - init - Restored ? ${restored}`);
            if(self._config.verbose) console.log(`Blockchain - Start background fetching missing Blockheaders`);//TODO fetch and emit event when finished!
            if(self._config.verbose) console.log(`Blockchain ready \n`)

            return resolve(true);
        });
    }
}