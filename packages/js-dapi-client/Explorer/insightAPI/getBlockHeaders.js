const _fetch = require('../../util/fetcher.js')._fetch;
exports.getBlockHeaders = function () {
    let self = this;
    return async function (identifier, nbOfBlocks = 25, direction = 1) {
        return new Promise(async function (resolve, reject) {
            let isHeight = false;
            //We accept two different possibilities for direction
            //either 1 : ascendant or -1 descendant.
            if (typeof(direction) === 'undefined' || direction.constructor.name !== "Number" || (direction !== 1 && direction !== -1)) direction = 1;
            //Un-necessary because already preset in function header
            if (typeof(nbOfBlocks) === 'undefined') nbOfBlocks = 25;
            //By default start at height 0
            if (typeof(identifier) === 'undefined') identifier = 0;
            if (identifier.constructor.name == "Number") {
                isHeight = true
            }
            if (direction === -1) {
                if (!isHeight) {
                    //This is a particular case to handle, we can't subtract from a hash, therefore we need to fetch the hash
                    identifier = await self.Explorer.API.getHeightFromHash(identifier);
                }
                identifier -= (nbOfBlocks - 1);
                //Just to be sure.
                if (identifier < 0) identifier = 0;
            }

            let getInsightCandidate = await self.Discover.getInsightCandidate();
            let getInsightURI = getInsightCandidate.URI;

            //Block-headers accept height or hash.
            let url = `${getInsightURI}/block-headers/${identifier}/${nbOfBlocks}`;
            _fetch({type: "GET", url: url}, function (err, data) {
                if (err) {
                    //This probably means that the getHeaders is not provided by the API (not updated version of insight API)
                    //Inform user of that
                    console.error(`The insight API provided by ${getInsightCandidate.URI} do not handle this feature.`);
                    return resolve(false);
                }
                if (data) {
                    return resolve(data);
                } else {
                    return resolve(null);
                }
            });
        });
    }
}
/*
 getBlockHeaders(hash|height, [nbBlocks,[direction]])
 */