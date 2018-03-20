/* eslint no-underscore-dangle: 0, no-console: 0 */
// TODO: Remove dangling underscores in this module's implementation
// TODO: Performance wise we might want to make Bluebird default for promise everywhere
const Promise = require('bluebird');
const Blkchain = require('./blockchain.js');
const socket = require('socket.io-client');

// TODO: Implement with (query, update) signature
const init = () => async () => new Promise((async () => {
  this.Blockchain.chain = {};

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
    isMainChain: true,
  };

  // Instantiate blockchain with genesis header, and assign to store on a inmem db.
  this.Blockchain.chain = await new Blkchain({ genesisHeader });
  const { chain } = this.Blockchain;
  this.Blockchain.isChainReady = true;
  if (this._config.verbose) {
    console.log('Blockchain - init - blockchain ready');
    console.log('Blockchain - init - selecting a socket to connect with');
  }
  const socketURI = (await this.Discover.getSocketCandidate()).URI;
  socket(socketURI, {
    reconnect: true,
    'reconnection delay': 500,
  });


  // Fetching last block
  const lastTip = await this.Explorer.API.getLastBlock();
  await chain.addHeader(lastTip);
  socket.on('connect', () => {
    socket.emit('subscribe', 'inv');
    socket.emit('subscribe', 'sync');
    if (this._config.verbose) console.log('Connected to socket -', socketURI);

    socket.on('block', async (_block) => {
      const blockHash = _block.toString();
      if (this._config.verbose) { console.log('Received Block', blockHash); }
      // Checkout the full block from Explorer (insightAPI)
      // TODO : We want this to be async.
      const block = await this.Explorer.API.getBlock(blockHash);
      await chain.addHeader(block);
      if (this._config.verbose) { console.log('tip is', await chain.tip); }
    });
  });
  if (this._config.verbose) { console.log('Blockchain ready'); }
}));

module.exports = { init };
