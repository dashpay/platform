const _fetch = require('../../util/fetcher.js')._fetch;
const axios = require('axios');
const explorerGet = require('../../Common/ExplorerHelper').explorerGet;

exports.getBlockHeaders = function(identifier = 0, nbOfBlocks = 25, isAscending = true) {

    return (!Number.isInteger(identifier) ?
        Promise.resolve(SDK.Explorer.API.getHeightFromHash(identifier)) :
        Promise.resolve(identifier))
        .then(height => {
            return explorerGet(`/block-headers/${Math.max(0, isAscending ? height : height - 1)}/${nbOfBlocks}`)
                .then(data => {
                    return data.headers;
                })
        })
};

// exports.getBlockHeaders = function() {
//     let self = this;
//     return async function(identifier, nbOfBlocks = 25, direction = 1) {
//         return new Promise(async function(resolve, reject) {
//             let isHeight = false;
//             //We accept two different possibilities for direction
//             //either 1 : ascendant or -1 descendant.
//             if (typeof (direction) === 'undefined' || direction.constructor.name !== "Number" || (direction !== 1 && direction !== -1)) direction = 1;
//             //Un-necessary because already preset in function header
//             if (typeof (nbOfBlocks) === 'undefined') nbOfBlocks = 25;
//             //By default start at height 0
//             if (typeof (identifier) === 'undefined') identifier = 0;
//             if (identifier.constructor.name == "Number") {
//                 isHeight = true
//             }
//             if (direction === -1) {
//                 if (!isHeight) {
//                     //This is a particular case to handle, we can't subtract from a hash, therefore we need to fetch the hash
//                     identifier = await self.Explorer.API.getHeightFromHash(identifier);
//                 }
//                 identifier -= (nbOfBlocks - 1);
//                 //Just to be sure.
//                 if (identifier < 0) identifier = 0;
//             }

//             let getConnectorCandidate = await self.Discover.getConnectorCandidate();
//             let getInsightURI = getConnectorCandidate.URI;

//             //Block-headers accept height or hash.
//             let url = `${getInsightURI}/block-headers/${identifier}/${nbOfBlocks}`;
//             return axios
//                 .get(url)
//                 .then(function(response) {
//                     if (response.hasOwnProperty('data') && response.data.hasOwnProperty('headers'))
//                         return resolve(response.data.headers);
//                     else
//                         return resolve(null);
//                 })
//                 .catch(function(error) {
//                     if (error) {
//                         //TODO : Signaling + removal feat
//                         console.error(`An error was triggered while fetching candidate ${getConnectorCandidate.idx} - signaling and removing from list`);
//                         return resolve(false);
//                     }
//                 });
//         });
//     }
// };