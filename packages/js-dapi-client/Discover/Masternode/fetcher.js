const has = require('../../util/has.js');
const { uuid } = require('khal');

exports.fetcher = function() {

    //Assume that this is a list of masternode fetched from an internal cache, or may be some starting point.
    const knownNodes = [];
    const INSIGHT_SEED = (SDK._config.DISCOVER.INSIGHT_SEEDS);
    if (!INSIGHT_SEED) {
        resolve(0);
    }
    for (let i = 0; i < INSIGHT_SEED.length; i++) {
        let elem = INSIGHT_SEED[i];
        let fullBase = `${elem.protocol}://${elem.base}:${elem.port}/`;
        let apiPath = elem.path;
        let socketPath = 'socket.io/?transport=websocket';
        knownNodes.push({ protocol: elem.protocol, port: elem.port, base: elem.base, fullBase: fullBase, insightPath: apiPath, socketPath: socketPath });
    }

    let unvalidatedMasternodeList = [].concat(knownNodes);

    return SDK.Discover.Masternode.validate(unvalidatedMasternodeList);
}