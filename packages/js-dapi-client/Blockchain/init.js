const has = require('../util/has.js');
const {uuid}=require('khal');
const Connector = require('../util/Connector');
const BSPVDash = require('blockchain-spv-dash');
const Promise = require('bluebird');//TODO Performance wise we might want to make Bluebird default for promise everywhere.
const levelup = require('levelup');
const db = levelup('dash.chain', {db: require('memdown')});
const EE2 = require('eventemitter2');
const {misc} = require('khal');
exports.init = function () {
    let self = this;
    return async function (params) {
        return new Promise(async function (resolve, reject) {
            self.Blockchain.emitter =  new EE2();
            let emitter = self.Blockchain.emitter;
            let genesisHeader = null;
            let listOfHeader = [];

            let defaultConfig = require('./config.js');
            const {merge} = require('khal').misc;
            const config = merge(params, defaultConfig);
            //Fetching last block
            let lastTip = await self.Explorer.API.getLastBlock();
            let lastHeight = lastTip.height;

            //If possible fetch the previous 100 (or specified) blocks.
            let fetchMultiple = null;
            if (config.fullFetch) {
                //Then we specifically fetch all block from one to last.

            } else {
                fetchMultiple = await self.Explorer.API.getBlockHeaders(lastHeight - 1, config.numberOfHeadersToFetch, -1);
            }
            if (fetchMultiple) {
                listOfHeader = fetchMultiple;
            }

            listOfHeader.push(lastTip);
            //Genesis is the oldest
            genesisHeader = listOfHeader[0];
            listOfHeader = (listOfHeader.slice(1, listOfHeader.length));
            //Set it as a genesis (even if we know it's not the case, that a requirement of BSPVDash.
            //Mind that the height will be wrong and that we won't be able to go before the designated block.
            //If you want, you can look for an alternate way in Blockchain/alternate which have not these limitation.
            self.params.blockchain.genesisHeader = self.Blockchain._normalizeHeader(genesisHeader);
            if (self._config.verbose) console.log(`Initialized blockchain at block ${genesisHeader.height} (${lastTip.height - genesisHeader.height} blocks ago) `);

            self.Blockchain.chain = new BSPVDash(self.params.blockchain, db, {ignoreCheckpoints: true}); // TODO: get rid of checkpoints
            if (listOfHeader.length > 0) {
                await self.Blockchain.addBlock(listOfHeader);
            }

            if (config.autoConnect) {
                let socketURI = (await self.Discover.getSocketCandidate()).URI;
                const socket = require('socket.io-client')(socketURI, {
                    'reconnect': true,
                    'reconnection delay': 500,
                });
                socket.on('connect', function () {
                    emitter.emit('socket.connected', socket);
                    socket.emit('subscribe', 'inv');
                    socket.emit('subscribe', 'sync');
                    if (self._config.verbose) console.log('Connected to socket -', socketURI);

                    socket.on('block', async function (_block) {
                        let blockHash = _block.toString();
                        emitter.emit('socket.block', blockHash);
                        // if (self._config.verbose) console.log('Received Block', blockHash);
                        //Checkout the full block from Explorer (insightAPI)
                        self.Explorer.API.getBlock(blockHash).then(function (block) {
                            if (block) {
                                self.Blockchain.addBlock([block]);
                            }
                        });
                        // await self.Blockchain.addBlock(block);
                        // let diff =  await self.Blockchain.expectNextDifficulty()
                        // console.log('Estimated next diff',diff);
                    });
                    socket.on('tx',function(tx){
                        emitter.emit('socket.tx', tx);
                    });
                });
            }
            if (self._config.verbose) console.log(`Blockchain ready \n`)
            emitter.emit('ready', true);
            return resolve(true);
        });
    }
}