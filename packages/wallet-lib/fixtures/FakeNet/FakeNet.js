const blocksData = require('./data/blocks/blocks.js');
const BaseTransporter = require('../../src/transporters/types/BaseTransporter/BaseTransporter');

const bestBlockDataHeight = 21546;
/**
 * This is a saved snapshot of some selected blocks and transactions
 * Meant to be used as replacement of DAPIClient.
 * Read more on the specificities on Readme.md and the things that are saved
 *
 */
class FakeNet extends BaseTransporter {
  constructor(props) {
    super({ ...props, type: 'DAPIClient' });

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

// FakeNet.prototype.getAddressSummary = require('./methods/getAddressSummary');
FakeNet.prototype.getBestBlock = require('./methods/getBestBlock');
FakeNet.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
FakeNet.prototype.getBestBlockHeader = require('./methods/getBestBlockHeader');
FakeNet.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
FakeNet.prototype.getBlockByHash = require('./methods/getBlockByHash');
FakeNet.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
FakeNet.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
FakeNet.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
FakeNet.prototype.getStatus = require('./methods/getStatus');
FakeNet.prototype.getAddressSummary = require('./methods/getAddressSummary');
FakeNet.prototype.getTransaction = require('./methods/getTransaction');
FakeNet.prototype.getUTXO = require('./methods/getUTXO');
FakeNet.prototype.sendTransaction = require('./methods/sendTransaction');
FakeNet.prototype.subscribeToAddressesTransactions = require('./methods/subscribeToAddressesTransactions');
FakeNet.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
FakeNet.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');

module.exports = FakeNet;
