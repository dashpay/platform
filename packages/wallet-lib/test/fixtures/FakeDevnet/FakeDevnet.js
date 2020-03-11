const blocksData = require('./data/blocks.js');
/**
 * This is a saved snapshot of some selected blocks and transactions
 * Meant to be used as replacement of DAPIClient.
 * Read more on the specificities on Readme.md and the things that are saved
 *
 */
class FakeDevnet {
  constructor() {
    this.height = Object.keys(blocksData.heights)[0];
    this.blockHash = blocksData.heights[this.height];

    this.relayFee = 0.00001;
    this.difficulty = 0.00171976818884149;
    this.network = 'testnet';

    this.blocks = blocksData;
  }

  getMnemonicList() {
    return [
      'nerve iron scrap chronic error wild glue sound range hurdle alter dwarf',
    ];
  }
}

// FakeDevnet.prototype.getAddressSummary = require('./methods/getAddressSummary');
FakeDevnet.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
FakeDevnet.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
FakeDevnet.prototype.getBlockByHash = require('./methods/getBlockByHash');
FakeDevnet.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
FakeDevnet.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
FakeDevnet.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
FakeDevnet.prototype.getStatus = require('./methods/getStatus');
// FakeDevnet.prototype.getTransaction = require('./methods/getTransaction');
// FakeDevnet.prototype.getUTXO = require('./methods/getUTXO');
// FakeDevnet.prototype.sendTransaction = require('./methods/sendTransaction');
// FakeDevnet.prototype.subscribeToAddressesTransactions = require('./methods/subscribeToAddressesTransactions');
// FakeDevnet.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
// FakeDevnet.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');
module.exports = FakeDevnet;
