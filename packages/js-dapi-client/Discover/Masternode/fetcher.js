const has = require('../../util/has.js');
const {uuid}=require('khal');

exports.fetcher = function() {
    let self = this;
    return async function () {
        return new Promise(async function (resolve, reject) {

            //Assume that this is a list of masternode fetched from an internal cache, or may be some starting point.
            const knownNodes = [];
            const INSIGHT_SEED = (self._config.DISCOVER.INSIGHT_SEEDS);
            if(!INSIGHT_SEED){
                return resolve(0);
            }
            for(let i = 0 ; i<INSIGHT_SEED.length; i++){
                knownNodes.push(INSIGHT_SEED[i].fullPath);
            }

            let unvalidatedMasternodeList = [].concat(knownNodes);

            let validMasternodeList = await (self.Discover.Masternode.validate(unvalidatedMasternodeList));
            return resolve(validMasternodeList);

        });
    }
}