const BlockStore = require('./blockstore');
const config = require('../config/config');
const ForkedChain = require('./forkedchain');
const Consensus = require('./consensus');
const utils = require('../lib/utils');

const SpvChain = class {
  constructor(chainType, customGenesis) {
    this.chainHeight = 0;
    this.forkedChains = [];
    this.POW = 1; // cumulative difficulty
    this.ready = false;

    switch (chainType) {
      case 'testnet':
        this.genesisHeader = config.getTestnetGenesis();
        break;
      case 'livenet':
        this.genesisHeader = config.getLivenetGenesis();
        break;
      case 'lowdiff':
        this.genesisHeader = config.getLowDiffGenesis();
        break;
      case 'custom_genesis':
        this.genesisHeader = utils.normalizeHeader(customGenesis);
        break;
      default:
        throw new Error('Unhandled chaintype');
    }

    this.store = new BlockStore(this.genesisHeader.hash);
    this.forkedChains.push(new ForkedChain(this.genesisHeader, this.POW));
  }

  getTipHash() {
    return this.getBestFork().getTipHash();
  }

  isChainReady() {
    return this.ready;
  }

  putStore(block) {
    this.POW += block.bits;
    this.chainHeight += 1;
    return this.store.put(block);
  }

  addCachedBlock(block) {
    const tipConnection = this.forkedChains.filter(fc => fc.isConnectedToTip(block));
    const headConnection = this.forkedChains.filter(fc => fc.isConnectedToHead(block));

    if (tipConnection.length > 0) {
      tipConnection[0].addTip(block);
    } else if (headConnection.length > 0) {
      headConnection[0].addHead(block);
    } else {
      this.forkedChains.push(new ForkedChain(block, this.POW, this.getTipHash()));
    }
  }

  getBestFork() {
    const maxDifficulty = Math.max(...this.forkedChains.map(f => f.getPOW()));
    return this.forkedChains.find(f => f.getPOW() === maxDifficulty);
  }

  getAllForks() {
    return this.forkedChains;
  }

  processMaturedChains() {
    const bestChainMaturedBlocks = this.getBestFork().getMaturedBlocks();

    for (let i = 0; i < bestChainMaturedBlocks.length; i += 1) {
      this.putStore(bestChainMaturedBlocks.pop());
    }

    // todo: kill expired chains
  }

  addHeader(header) {
    const headerNormalised = utils.normalizeHeader(header);

    if (Consensus.isValidBlockHeader(this.getBestFork().blocks, headerNormalised)) {
      this.addCachedBlock(headerNormalised);
      this.processMaturedChains();
    } else {
      throw new Error('Block does not conform to header consensus rules');
    }
  }

  addHeaders(headers) {
    const self = this;
    headers.forEach((header) => {
      self.addHeader(header);
    });
  }

  getChainHeight() {
    return this.chainHeight + (this.getBestFork().getForkHeight() - 1);
  }

  getBlock(blockhash) {
    return this.store.get(blockhash)
      .then((blockInDB) => {
        if (blockInDB) {
          return blockInDB;
        }
        const blockInFork = this.getBestFork().blocks.filter(b => b.hash === blockhash);
        if (blockInFork.length === 1) {
          return blockInFork[0];
        }
        return null;
      });
  }
};

module.exports = SpvChain;
