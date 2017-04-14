const has = require('../util/has.js');
const {uuid}=require('khal');
const Connector = require('../util/Connector');
const BSPVDash = require('blockchain-spv-dash');
const Promise = require('bluebird');//TODO Performance wise we might want to make Bluebird default for promise everywhere.
const levelup = require('levelup');
const db = levelup('dash.chain', { db: require('memdown') });

exports.init = function () {
    let self = this;
    return async function (query, update) {
        return new Promise(async function (resolve, reject) {
            self.Blockchain.chain = {};

            let genesisHeader = null;
            let listOfHeader = [];

            //Fetching last block
            let lastTip = await self.Explorer.API.getLastBlock();
            let lastHeight = lastTip.height;

            //If possible fetch the previous 50 blocks.
            let fetchMultiple = await self.Explorer.API.getBlockHeaders(lastHeight-1,50,-1);
            // let fetchMultiple = false;
            if(fetchMultiple) {
                listOfHeader = fetchMultiple;
            }

            listOfHeader.push(lastTip);
            listOfHeader = listOfHeader.map(function(_bh){
                return self.Blockchain._normalizeHeader(_bh)
            });
            //Genesis is the oldest
             genesisHeader=self.Blockchain._normalizeHeader(listOfHeader[0]);
            listOfHeader=(listOfHeader.slice(1,listOfHeader.length));
            //Set it as a genesis (even if we know it's not the case, that a requirement of BSPVDash.
            //Mind that the height will be wrong and that we won't be able to go before the designated block.
            //If you want, you can look for an alternate way in Blockchain/alternate which have not these limitation.
            self.params.blockchain.genesisHeader = self.Blockchain._normalizeHeader(genesisHeader);
            var chain = new BSPVDash(self.params.blockchain, db, { ignoreCheckpoints: true }); // TODO: get rid of checkpoints

            chain.addHeaders(listOfHeader,function(err){
                if(err) console.error(err);
            });

            let socketURI = (await self.Discover.getSocketCandidate()).URI;
            const socket = require('socket.io-client')(socketURI, {
                'reconnect': true,
                'reconnection delay': 500,
            });
            socket.on('connect', function () {
                socket.emit('subscribe', 'inv');
                socket.emit('subscribe', 'sync');
                if (self._config.verbose) console.log('Connected to socket -', socketURI);

                socket.on('block', async function (_block) {
                    let blockHash = _block.toString();
                    if (self._config.verbose) console.log('Received Block', blockHash);
                    //Checkout the full block from Explorer (insightAPI)
                    //TODO : We want this to be async.
                    let block = await self.Explorer.API.getBlock(blockHash);
                    if(block){
                        block=self.Blockchain._normalizeHeader(block);
                        chain.addHeaders([block],function(err){
                            if(err) console.error(err);
                        });
                    }

                    // await self.Blockchain.addBlock(block);
                    // let diff =  await self.Blockchain.expectNextDifficulty()
                    // console.log('Estimated next diff',diff);
                });
                // socket.on('tx',function(tx){
                //     console.log('Received TX',tx);
                // });
            });

            // use the blockchain
            // })

            /*self.Blockchain.blocks={};
             let socketConf = {
             CONNECTOR_TYPE: "CLIENT",
             CONNECTOR_PATH: socketURI
             };



             // if(!socketOpened){
             //    if(self._config.errors) console.error(`Socket - Couldn't connect to any MN`);
             // }
             // let restored = await self.Blockchain.restore();
             // if(self._config.verbose) console.log(`Blockchain - init - Restored ? ${restored}`);
             // if(self._config.verbose) console.log(`Blockchain - Start background fetching missing Blockheaders`);//TODO fetch and emit event when finished!
             if (self._config.verbose) console.log(`Blockchain ready \n`)

             return resolve(true);*/
        });
    }
}