const BlockStore = require('./blockstore');
const config = require('../config/config');
const Consensus = require('./consensus');
const utils = require('../lib/utils');

const SpvChain = class {
  constructor(chainType, confirms = 100, startBlock) {
    this.root = null;
    this.allBranches = [];
    this.orphanBlocks = [];
    this.orphanChunks = [];
    this.confirmsBeforeFinal = confirms;
    this.init(chainType, startBlock);
    this.store = new BlockStore();
  }

  init(chainType, startBlock) {
    switch (chainType) {
      case 'testnet':
        this.network = 'testnet';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getTestnetGenesis();
        break;
      case 'devnet':
        this.network = 'devnet';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getDevnetGenesis();
        break;
      case 'livenet':
        this.network = 'mainnet';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getLivenetGenesis();
        break;
      case 'mainnet':
        this.network = 'mainnet';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getLivenetGenesis();
        break;
      case 'lowdiff':
        this.network = 'regtest';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getLowDiffGenesis();
        break;
      default:
        if (startBlock) {
          this.root = startBlock;
          this.network = 'mainnet';
        } else {
          throw new Error('Unhandled chaintype or startBlock not provided');
        }
        break;
    }
    this.root.children = [];
    this.setAllBranches();
  }

  getLongestChain() {
    return this.allBranches.sort((b1, b2) => b1 < b2)[0];
  }

  checkPruneBlocks() {
    const longestChain = this.getLongestChain();

    if (longestChain.length > this.confirmsBeforeFinal) {
      const pruneBlock = longestChain.splice(0, 1)[0];
      // Children discarded as stale branches
      delete pruneBlock.orphan;
      this.store.put(pruneBlock);
    }
  }

  getTipHash() {
    return this.getLongestChain().slice(-1)[0].hash;
  }

  getTipHeader() {
    return this.getLongestChain().slice(-1)[0];
  }

  getHeader(hash) {
    return this.store.get(hash)
      .then((blockInDB) => {
        if (blockInDB) {
          return blockInDB;
        }

        return this.getLongestChain().filter(h => h.hash === hash)[0];
      });
  }

  findConnection(newHeader) {
    const stack = [this.root];
    while (stack.length > 0) {
      const node = stack.pop();
      if (node.hash === utils.getCorrectedHash(newHeader.prevHash)) {
        return node;
      }
      node.children.forEach((c) => { stack.push(c); });
    }
    return null;
  }

  setAllBranches(node = this.root, branch = []) {
    this.allBranches = [];
    branch.push(node);

    node.children.forEach((c) => {
      this.setAllBranches(c, Array.from(branch));
    });

    if (node.children.length === 0) {
      this.allBranches.push(branch);
    }
  }

  appendHeadersToLongestChain(headers) {
    const newLongestChain = this.getLongestChain().concat(headers);
    this.allBranches = [];
    this.allBranches.push(newLongestChain);
  }

  getAllBranches() {
    return this.allBranches;
  }

  isDuplicate(compareHash) {
    return this.getAllBranches().map(branch => branch.map(node => node.hash))
      .concat(this.orphanBlocks.map(orphan => orphan.hash))
      .filter(hash => hash === compareHash).length > 0;
  }

  orphanReconnect() {
    for (let i = 0; i < this.orphanBlocks.length; i += 1) {
      const connectionTip = this.findConnection(this.orphanBlocks[i]);
      if (connectionTip) {
        connectionTip.children.push(this.orphanBlocks[i]);
        this.orphanBlocks.splice(i, 1);
      }
    }
  }

  getOrphans() {
    return this.orphanBlocks;
  }

  processValidHeader(header) {
    const connection = this.findConnection(header);
    if (connection) {
      connection.children.push(header);
      this.orphanReconnect();
    } else {
      this.orphanBlocks.push(header);
    }
  }

  addHeader(header) {
    const headerNormalised = utils.normalizeHeader(header);

    if (this.isValid(headerNormalised, this.getLongestChain())) {
      headerNormalised.children = [];
      this.processValidHeader(headerNormalised);
      this.setAllBranches();
      this.checkPruneBlocks();
      return true;
    }
    return false;
  }

  /* eslint-disable no-param-reassign */
  static isParentChild(header, previousHeader) {
    if (utils.getCorrectedHash(header.prevHash) !== previousHeader.hash) {
      return false;
    }
    if (!header.children) {
      header.children = [];
    }
    if (!previousHeader.children) {
      previousHeader.children = [];
    }
    previousHeader.children.push(header);
    return true;
  }
  /* eslint-enable no-param-reassign */

  isValid(header, previousHeaders) {
    return !!(Consensus.isValidBlockHeader(header, previousHeaders, this.network)
      && !this.isDuplicate(header.hash));
  }

  addHeaders(headers) {
    const self = this;
    const normalizedHeaders = headers.map(h => utils.normalizeHeader(h));
    const isOrphan = !SpvChain.isParentChild(normalizedHeaders[0], this.getTipHeader());

    const allValid = normalizedHeaders.reduce(
      (acc, header, index, array) => {
        const previousHeaders = normalizedHeaders.slice(0, index);
        if (index !== 0) {
          if (!SpvChain.isParentChild(header, array[index - 1])
            || !self.isValid(header, previousHeaders)) {
            throw new Error('Some headers are invalid');
          }
          return acc && true;
        }
        if (!self.isValid(header, self.getLongestChain())) {
          throw new Error('Some headers are invalid');
        }
        return acc && true;
      }, true,
    );
    if (!allValid) {
      throw new Error('Some headers are invalid');
    }
    if (isOrphan) {
      this.orphanChunks.push(headers);
    } else {
      self.appendHeadersToLongestChain(normalizedHeaders);
    }
    this.checkPruneBlocks();
  }
};

module.exports = SpvChain;
