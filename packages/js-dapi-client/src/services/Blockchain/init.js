/* eslint-disable */
// TODO: Make this file pass linting!
// TODO: Performance wise we might want to make Bluebird default for promise everywhere.
const Promise = require('bluebird');
const levelup = require('levelup');
const memdown = require('memdown');

const db = levelup('dash.chain', { db: memdown });
const EE2 = require('eventemitter2');

let listOfHeader = [];
let lastTip = null;
const fetchAndAdd = async (startHeight, numberOfBlock) => {
  const blockHeaders = await this.Explorer.API.getBlockHeaders(startHeight, numberOfBlock, 1);
  await this.Blockchain.addBlock(blockHeaders);
  console.log(blockHeaders[0].height, blockHeaders[blockHeaders.length - 1].height);
  return startHeight + numberOfBlock;
};
const startSocketConnection = async (config) => {
  const { emitter } = this.Blockchain;
  const socketURI = (await this.Discover.getSocketCandidate()).URI;
  const socket = require('socket.io-client')(socketURI, {
    reconnect: true,
    'reconnection delay': 500,
  });
  socket.on('connect', () => {
    emitter.emit('socket.connected', socket);
    socket.emit('subscribe', 'inv');
    socket.emit('subscribe', 'sync');
    if (this._config.verbose) console.log('Connected to socket -', socketURI);

    socket.on('block', async (_block) => {
      const blockHash = _block.toString();
      emitter.emit('socket.block', blockHash);
      // if (this._config.verbose) console.log('Received Block', blockHash);
      // Checkout the full block from Explorer (insightAPI)

      if (config.socket.autoAddBlock) {
        this.Explorer.API.getBlock(blockHash).then((block) => {
          if (block) {
            this.Blockchain.addBlock([block]);
          }
        });
      }

      // await this.Blockchain.addBlock(block);
      // let diff =  await this.Blockchain.expectNextDifficulty()
      // console.log('Estimated next diff',diff);
    });
    socket.on('tx', (tx) => {
      emitter.emit('socket.tx', tx);
    });
  });
};
const startFullFetch = async (startingBlockHeight) => {
  const genesisHeader = await this.Explorer.API.getBlock(startingBlockHeight);
  this.params.blockchain.genesisHeader = genesisHeader;
  const startHeight = startingBlockHeight + 1;

  this.Blockchain.emitter.once('chain.ready', async () => {
    const processFullFetching = async (startHeight, numberOfBlock) => {
      startHeight = await fetchAndAdd(this, startHeight, numberOfBlock);
      if (startHeight < (lastTip.height - numberOfBlock)) {
        await processFullFetching(this, startHeight, numberOfBlock);
      } else {
        // Because multiple blocks might have been created while we start our fullfetch.
        lastTip = await this.Explorer.API.getLastBlock();
        if (startHeight < lastTip.height) {
          await processFullFetching(this, startHeight, (lastTip.height - startHeight) + 1);
        }
        return true;
      }
    };
    await processFullFetching(this, startHeight, 100);
    // console.log(this.Blockchain.chain);
    console.log('chain is ready');
  });
  // const fetchAndAdd:
  return false;
};
const startSmartFetch = async (config) => {
  const superblockCycle = 16616;

  // Will return the last nbOfSuperblock from Height
  const lastSuperblocksList = function (_height, nbOfSuperblock) {
    const superblockHeightList = [];

    let superblock = _height - (_height % superblockCycle) + superblockCycle;// next superblock
    while (nbOfSuperblock--) {
      superblock -= superblockCycle;
      superblockHeightList.push(superblock);
    }
    superblockHeightList.sort();

    return superblockHeightList;
  };
  const startingHeight = lastSuperblocksList(lastTip.height, 2)[0];
  await startFullFetch(this, startingHeight);
  return false;
};
const startQuickFetch = async (config) => {
  if (!config || !config.numberOfHeadersToFetch) { throw new Error('Missing config. Error.'); }
  // Fetching last block
  const lastHeight = lastTip.height;
  const blockHeaders =
    await this.Explorer.API.getBlockHeaders(lastHeight - 1, config.numberOfHeadersToFetch, false);
  if (!blockHeaders || blockHeaders.length < 1) {
    console.log(blockHeaders);
    throw new Error('Missing block. Initialization impossible.');
  }
  blockHeaders.push(lastTip);
  this.params.blockchain.genesisHeader = blockHeaders[0];
  listOfHeader = (blockHeaders.slice(1, blockHeaders.length));
  return true;
};
const init = () => async params => new Promise((async (resolve) => {
  this.Blockchain.emitter = new EE2();
  const { emitter } = this.Blockchain;

  const defaultConfig = require('./config.js');
  const { merge } = require('khal').misc;
  const config = merge(params, defaultConfig);

  // We get the last Block generated.
  lastTip = await this.Explorer.API.getLastBlock();

  if (config.fullFetch) {
    // Then we specifically fetch all block from one to last.
    // 50min for fullFetch on testnet
    // 3hr on livenet
    await startFullFetch(this, 0);
  } else if (config.smartFetch) {
    // Then we fetch using our smart fetching (superblock based)
    // Between 5 and 7 minute on livenet or testnet
    await startSmartFetch(this);
  } else {
    // Then we do a lazy fetching : last X block.
    // Depend on Xblock
    await startQuickFetch(this, config);
  }

  // Set it as a genesis (even if we know it's not the case, that a requirement of BSPVDash.
  // Mind that the height will be wrong and that we won't be able to go before the designated
  // block. If you want, you can look for an alternate way in Blockchain/alternate which have
  // not these limitations.
  const genesisHeader = this.params.blockchain.genesisHeader;
  if (!genesisHeader) { throw new Error('Missing Genesis Header. Dropping init.'); }

  this.params.blockchain.genesisHeader = this.Blockchain._normalizeHeader(genesisHeader);
  if (this._config.verbose) console.log(`Initialized blockchain at block ${genesisHeader.height || 0} (${lastTip.height - (genesisHeader.height || 0)} blocks ago) `);

  // Start Blockchain-spv-dash
  this.Blockchain.chain = new BSPVDash(this.params.blockchain, db, { ignoreCheckpoints: true }); // TODO: get rid of checkpoints
  emitter.emit('chain.ready', true);
  // If provided so, add the list of header inside the blockchain
  if (listOfHeader && listOfHeader.length > 0) {
    console.log('Added ', listOfHeader.length, 'blocks');
    await this.Blockchain.addBlock(listOfHeader);
  }
  if (config.socket.autoConnect) {
    startSocketConnection(this, config);
  }
  if (this._config.verbose) console.log('Blockchain ready \n');
  emitter.emit('ready', true);
  return resolve(true);
}));

module.exports = { init };
