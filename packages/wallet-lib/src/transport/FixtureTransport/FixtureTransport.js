const blocksData = require('./data/blocks/blocks.js');
const AbstractTransport = require('../AbstractTransport');

const bestBlockDataHeight = 21546;

/**
 * This is a saved snapshot of some selected blocks and transactions
 * Meant to be used as replacement of DAPIClientTransport.
 * Read more on the specificities on Readme.md and the things that are saved
 *
 */
class FixtureTransport extends AbstractTransport {
  constructor() {
    super();

    this.height = bestBlockDataHeight;
    this.blockHash = blocksData.heights[this.height];

    this.relayFee = 0.00001;
    this.difficulty = 0.00171976818884149;
    this.network = 'testnet';
  }

  setHeight(height) {
    if (!height) throw new Error('Height needed');
    this.height = height;

    if (!blocksData.heights[this.height]) {
      throw new Error(`Missing block ${this.height}`);
    }
    this.blockHash = blocksData.heights[this.height];
  }

  rewindBlock(step = 1) {
    this.height -= step;
    if (!blocksData.heights[this.height]) {
      throw new Error(`Missing block ${this.height}`);
    }
    this.blockHash = blocksData.heights[this.height];
  }

  forwardBlock(step = 1) {
    this.height += step;
    if (!blocksData.heights[this.height]) {
      throw new Error(`Missing block ${this.height}`);
    }
    this.blockHash = blocksData.heights[this.height];
  }

  // eslint-disable-next-line class-methods-use-this
  getMnemonicList() {
    return [
      'nerve iron scrap chronic error wild glue sound range hurdle alter dwarf',
    ];
  }
}

// FixtureTransport.prototype.getAddressSummary = require('./methods/getAddressSummary');
FixtureTransport.prototype.getBestBlock = require('./methods/getBestBlock');
FixtureTransport.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
FixtureTransport.prototype.getBestBlockHeader = require('./methods/getBestBlockHeader');
FixtureTransport.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
FixtureTransport.prototype.getBlockByHash = require('./methods/getBlockByHash');
FixtureTransport.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
FixtureTransport.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
FixtureTransport.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
FixtureTransport.prototype.getStatus = require('./methods/getStatus');
FixtureTransport.prototype.getAddressSummary = require('./methods/getAddressSummary');
FixtureTransport.prototype.getTransaction = require('./methods/getTransaction');
FixtureTransport.prototype.getUTXO = require('./methods/getUTXO');
FixtureTransport.prototype.sendTransaction = require('./methods/sendTransaction');
FixtureTransport.prototype.subscribeToAddressesTransactions = require('./methods/subscribeToAddressesTransactions');
FixtureTransport.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
FixtureTransport.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');

module.exports = FixtureTransport;
