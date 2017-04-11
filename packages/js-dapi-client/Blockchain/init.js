const has = require('../util/has.js');
const {uuid}=require('khal');

exports.init = function() {
    let self = this;
    return async function(query, update){
        return new Promise(async function (resolve, reject) {
            if(self._config.verbose) console.log('Blockchain - init - try to restore Blockchain state');
            let restored = await self.Blockchain.restore();
            if(self._config.verbose) console.log(`Blockchain - init - Restored ? ${restored}`);
            let lastHeight = await self.Explorer.API.getLastBlockHeight();
            if(self._config.verbose) console.log(`Blockchain - Last block height is ${lastHeight}.`);
            if(self._config.verbose) console.log(`Blockchain - Start fetching missing Blockheaders`);
            if(self._config.verbose) console.log(`Blockchain ready \n`)

            return resolve(true);
        });
    }
}