const has = require('../../util/has.js');
const {uuid}=require('khal');
const Connector = require('../../util/Connector');
const BSPVDash = require('blockchain-spv-dash');
const Promise = require('bluebird');//TODO Performance wise we might want to make Bluebird default for promise everywhere.

exports.init = function () {
    let self = this;
    return async function (query, update) {
        return new Promise(async function (resolve, reject) {
            self.Blockchain.chain = {};

            const genesisHeader = {
                hash: '00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6',
                confirmations: 652866,
                size: 306,
                height: 0,
                version: 1,
                merkleroot: 'e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7',
                tx: ['e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7'],
                time: 1390095618,
                mediantime: 1390095618,
                nonce: 28917698,
                bits: '1e0ffff0',
                difficulty: 0.000244140625,
                chainwork: '0000000000000000000000000000000000000000000000000000000000100010',
                nextblockhash: '000007d91d1254d60e2dd1ae580383070a4ddffa4c64c2eeb4a2f9ecc0414343',
                isMainChain: true
            };
            // const block1 = { hash: '000007d91d1254d60e2dd1ae580383070a4ddffa4c64c2eeb4a2f9ecc0414343',
            //     confirmations: 652883,
            //     size: 186,
            //     height: 1,
            //     version: 2,
            //     merkleroot: 'ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7',
            //     tx: [ 'ef3ee42b51e2a19c4820ef182844a36db1201c61eb0dec5b42f84be4ad1a1ca7' ],
            //     time: 1390103681,
            //     mediantime: 1390103681,
            //     nonce: 128987,
            //     bits: '1e0ffff0',
            //     difficulty: 0.000244140625,
            //     chainwork: '0000000000000000000000000000000000000000000000000000000000200020',
            //     previousblockhash: '00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6',
            //     nextblockhash: '00000bafcc571ece7c5c436f887547ef41b574e10ef7cc6937873a74ef1efeae',
            //     isMainChain: true,
            //     cbvalue: 500 };

            //Instantiate blockchain with genesis header, and assign to store on a inmem db.
            const Blkchain = require('./blockchain.js');
            self.Blockchain.chain = await new Blkchain({genesisHeader:genesisHeader});
            let chain = self.Blockchain.chain;
            // console.log(await chain.getBlock(0));
            // console.log(await chain.getBlock("00000ffd590b1485b3caadc19b22e6379c733355108f107a430458cdf3407ab6"));
            // await chain.addHeader(block1);
            // console.log(await chain.getBlock(1));
            self.Blockchain.isChainReady = true;
            if (self._config.verbose) console.log('Blockchain - init - blockchain ready');
            if (self._config.verbose) console.log('Blockchain - init - selecting a socket to connect with');
            let socketURI = (await self.Discover.getSocketCandidate()).URI;
            const socket = require('socket.io-client')(socketURI, {
                'reconnect': true,
                'reconnection delay': 500,
            });


            //Fetching last block
            let lastTip = await self.Explorer.API.getLastBlock();
            await chain.addHeader(lastTip);
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
                    await chain.addHeader(block);
                    console.log("tip is",await chain.tip);
                });
                // socket.on('tx',function(tx){
                //     console.log('Received TX',tx);
                // });
            });
             if (self._config.verbose) console.log(`Blockchain ready \n`)

        });
    }
}