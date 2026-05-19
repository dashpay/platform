import blocksData from './data/blocks/blocks.js';
import AbstractTransport from '../AbstractTransport.js';

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

import _FixtureTransport_getBestBlock from './methods/getBestBlock.js';
FixtureTransport.prototype.getBestBlock = _FixtureTransport_getBestBlock;
import _FixtureTransport_getBestBlockHash from './methods/getBestBlockHash.js';
FixtureTransport.prototype.getBestBlockHash = _FixtureTransport_getBestBlockHash;
import _FixtureTransport_getBestBlockHeader from './methods/getBestBlockHeader.js';
FixtureTransport.prototype.getBestBlockHeader = _FixtureTransport_getBestBlockHeader;
import _FixtureTransport_getBestBlockHeight from './methods/getBestBlockHeight.js';
FixtureTransport.prototype.getBestBlockHeight = _FixtureTransport_getBestBlockHeight;
import _FixtureTransport_getBlockByHash from './methods/getBlockByHash.js';
FixtureTransport.prototype.getBlockByHash = _FixtureTransport_getBlockByHash;
import _FixtureTransport_getBlockByHeight from './methods/getBlockByHeight.js';
FixtureTransport.prototype.getBlockByHeight = _FixtureTransport_getBlockByHeight;
import _FixtureTransport_getBlockHeaderByHash from './methods/getBlockHeaderByHash.js';
FixtureTransport.prototype.getBlockHeaderByHash = _FixtureTransport_getBlockHeaderByHash;
import _FixtureTransport_getBlockHeaderByHeight from './methods/getBlockHeaderByHeight.js';
FixtureTransport.prototype.getBlockHeaderByHeight = _FixtureTransport_getBlockHeaderByHeight;
import _FixtureTransport_getBlockchainStatus from './methods/getBlockchainStatus.js';
FixtureTransport.prototype.getBlockchainStatus = _FixtureTransport_getBlockchainStatus;
import _FixtureTransport_getTransaction from './methods/getTransaction.js';
FixtureTransport.prototype.getTransaction = _FixtureTransport_getTransaction;
import _FixtureTransport_sendTransaction from './methods/sendTransaction.js';
FixtureTransport.prototype.sendTransaction = _FixtureTransport_sendTransaction;
import _FixtureTransport_subscribeToAddressesTransactions from './methods/subscribeToAddressesTransactions.js';
FixtureTransport.prototype.subscribeToAddressesTransactions = _FixtureTransport_subscribeToAddressesTransactions;

export default FixtureTransport;