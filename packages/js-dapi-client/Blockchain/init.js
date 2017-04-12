const has = require('../util/has.js');
const {uuid}=require('khal');
const Connector = require('../util/Connector');
exports.init = function () {
    let self = this;
    return async function (query, update) {
        return new Promise(async function (resolve, reject) {
            if (self._config.verbose) console.log('Blockchain - init - try to restore Blockchain state');
            let socketURI = (await self.Discover.getSocketCandidate()).URI;
            let socketConf = {
                CONNECTOR_TYPE: "CLIENT",
                CONNECTOR_PATH: socketURI
            };
            const socket = require('socket.io-client')(socketURI, {
                'reconnect': true,
                'reconnection delay': 500,
            });

            socket.on('connect', function() {
                socket.emit('subscribe', 'inv');
                socket.emit('subscribe', 'sync');
                if(self._config.verbose)  console.log('Connected to socket -',socketURI);
                socket.on('block', function (block) {
                    var blockHash = block.toString();
                    if (self._config.verbose) console.log('Received Block', block);
                    console.log(blockHash);
                });
                // socket.on('tx',function(tx){
                //     console.log('Received TX',tx);
                // });
            });

            // if(!socketOpened){
            //    if(self._config.errors) console.error(`Socket - Couldn't connect to any MN`);
            // }
            // let restored = await self.Blockchain.restore();
            // if(self._config.verbose) console.log(`Blockchain - init - Restored ? ${restored}`);
            // if(self._config.verbose) console.log(`Blockchain - Start background fetching missing Blockheaders`);//TODO fetch and emit event when finished!
            if (self._config.verbose) console.log(`Blockchain ready \n`)

            return resolve(true);
        });
    }
}