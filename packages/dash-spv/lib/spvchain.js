const config = require('../config/config');
const Consensus = require('./consensus');
const utils = require('./utils');

const SpvChain = class {
  // TODO: add startBlockHeight as well
  constructor(chainType, confirms = 100, startBlock = null, startBlockHeight = 0) {
    this.root = null;
    this.allBranches = [];
    this.orphanBlocks = [];
    this.orphanChunks = [];
    this.confirmsBeforeFinal = confirms;
    this.init(chainType, startBlock);
    this.prunedHeaders = [];
    this.startBlockHeight = startBlockHeight;
    // TODO: test
    this.hashesByHeight = {
      [startBlockHeight]: this.root.hash,
    };
    this.heightByHash = {
      [this.root.hash]: startBlockHeight,
    };
    /**
     * Index set to check for duplicates
     * @type {Set<any>}
     */
    this.orphansHashes = new Set();
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
      case 'regtest':
        this.network = 'regtest';
        if (startBlock) {
          this.root = startBlock;
          break;
        }
        this.root = config.getRegtestGenesis();
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
    // this.root.children = [];
    this.setAllBranches();
  }

  /**
   * @param {BlockHeader} header
   * @param {number} height
   */
  makeNewChain(header, height) {
    this.allBranches = [[header]];
    this.startBlockHeight = height;
    this.hashesByHeight = {
      [height]: header.hash,
    };
    this.heightByHash = {
      [header.hash]: height,
    };
  }

  /** @private */
  checkPruneBlocks() {
    const longestChain = this.getLongestChain();

    longestChain
      .splice(0, longestChain.length - this.confirmsBeforeFinal)
      .forEach((header) => {
        this.prunedHeaders.push(header);
      });
  }

  /** @private */
  findConnection(newHeader) {
    const stack = [this.root];
    while (stack.length > 0) {
      const node = stack.pop();
      if (node.hash === utils.getCorrectedHash(newHeader.prevHash)) {
        return node;
      }
      // node.children.forEach((c) => { stack.push(c); });
    }
    return null;
  }

  /** @private */
  setAllBranches(node = this.root, branch = []) {
    this.allBranches = [];
    branch.push(node);

    // node.children.forEach((c) => {
    //   this.setAllBranches(c, Array.from(branch));
    // });

    // if (node.children.length === 0) {
    this.allBranches.push(branch);
    // }
  }

  /** @private */
  appendHeadersToLongestChain(headers) {
    const longestChain = this.getLongestChain();
    const lastHeight = this.startBlockHeight
      + longestChain.length + this.prunedHeaders.length;

    headers.forEach((header, i) => {
      const height = lastHeight + i;
      this.hashesByHeight[height] = header.hash;
      this.heightByHash[header.hash] = height;
    });

    const newLongestChain = longestChain.concat(headers);

    this.allBranches = [];
    this.allBranches.push(newLongestChain);
  }

  /** @private */
  getAllBranches() {
    return this.allBranches;
  }

  /** @private */
  isDuplicate(hash) {
    return !!this.heightByHash[hash] || this.orphansHashes.has(hash);
  }

  /** @private */
  orphanReconnect() {
    // for (let i = 0; i < this.orphanBlocks.length; i += 1) {
    // const connectionTip = this.findConnection(this.orphanBlocks[i]);
    // if (connectionTip) {
    //   connectionTip.children.push(this.orphanBlocks[i]);
    //   this.orphanBlocks.splice(i, 1);
    // }
    // }
  }

  /** @private */
  orphanChunksReconnect() {
    // TODO: consider optimizing with map of { [chunkHeadHash]: chunkIndex }
    // to get rid of sorting and make the whole function of O(n) complexity
    this.orphanChunks.sort((a, b) => a[0].timestamp - b[0].timestamp);

    for (let i = 0; i < this.orphanChunks.length; i += 1) {
      const chunk = this.orphanChunks[i];
      if (this.getTipHash() === utils.getCorrectedHash(chunk[0].prevHash)) {
        this.appendHeadersToLongestChain(chunk);
        chunk.forEach((header) => {
          this.orphansHashes.delete(header.hash);
        });
        this.orphanChunks.splice(i, 1);
        i -= 1;
      }
    }
  }

  /** @private */
  getOrphans() {
    return this.orphanBlocks;
  }

  /** @private */
  getOrphanChunks() {
    return this.orphanChunks;
  }

  /** @private */
  processValidHeader(header) {
    const connection = this.findConnection(header);
    if (connection) {
      // connection.children.push(header);
      this.orphanReconnect();
    } else {
      this.orphanBlocks.push(header);
    }
  }

  /** @private
   * validates a dashcore.BlockHeader object
   *
   * @param {Object} header
   * @param {Object[]} previousHeaders
   * @return {boolean}
   */
  isValid(header, previousHeaders) {
    const validBlockHeader = Consensus.isValidBlockHeader(header, previousHeaders, this.network);
    const duplicate = this.isDuplicate(header.hash);
    return !!validBlockHeader && !duplicate;
  }

  /* eslint-disable no-param-reassign */
  /**
   * verifies the parent child connection
   * between two adjacent dashcore.BlockHeader objects
   *
   * @param {Object} header
   * @param {Object} previousHeader
   * @return {boolean}
   */
  static isParentChild(header, previousHeader) {
    if (utils.getCorrectedHash(header.prevHash) !== previousHeader.hash) {
      return false;
    }
    // if (!header.children) {
    //   header.children = [];
    // }
    // if (!previousHeader.children) {
    //   previousHeader.children = [];
    // }
    // previousHeader.children.push(header);
    return true;
  }
  /* eslint-enable no-param-reassign */

  /**
   * Returns the longest chain of headers
   * @param options
   * @returns {*}
   */
  getLongestChain(options = { withPruned: false }) {
    let longestChain = this.allBranches.sort((b1, b2) => b1 < b2)[0];

    if (options.withPruned) {
      longestChain = this.prunedHeaders.concat(longestChain);
    }

    return longestChain;
  }

  /**
   * gets the block hash of the longest chain tip
   *
   * @return {string} hash
   */
  getTipHash() {
    return this.getLongestChain().slice(-1)[0].hash;
  }

  /**
   * gets the dashcore.BlockHeader object the longest chain tip
   *
   * @return {Object} header
   */
  getTipHeader() {
    return this.getLongestChain().slice(-1)[0];
  }

  /**
   * gets the dashcore.BlockHeader object for a specific block hash
   *
   * @param {string} hash
   * @return {Object} header
   */
  getHeader(hash) {
    // TODO: perform lookup in pruned headers?
    return this.getLongestChain().filter((h) => h.hash === hash)[0];
  }

  /**
   * Gets specified amount of headers in the confirmed chain
   * @param n
   * @return Object[]
   */
  getLastHeaders(n) {
    const longestChain = this.getLongestChain();
    let headers = longestChain.slice(-n);

    if (headers.length < n) {
      const remaining = n - headers.length;
      headers = [...this.prunedHeaders.slice(-remaining), ...headers];
    }

    return headers;
  }

  /**
   * adds a valid header to the tip of the longest spv chain.
   * If it cannot be connected to the tip it gets temporarily
   * added to an orphan array for possible later reconnection
   *
   * @param {Object[]|string[]|buffer[]} header
   * @return {boolean}
   */
  addHeader(header) {
    const headerNormalised = utils.normalizeHeader(header);

    if (this.isValid(headerNormalised, this.getLongestChain())) {
      // headerNormalised.children = [];
      this.processValidHeader(headerNormalised);
      this.setAllBranches();
      this.checkPruneBlocks();
      return true;
    }
    return false;
  }

  /**
   * adds an array of valid headers to the longest spv chain.
   * If they cannot be connected to last tip they get temporarily
   * added to an orphan array for possible later reconnection
   *
   * @param {Object[]|string[]|buffer[]} headers
   * @return {BlockHeader[]}
   */
  addHeaders(headers) {
    // TODO: fix. `addHeader` function uses partially implemented
    // reorg functionality and throws an error
    // if (headers.length === 1) {
    //   if (!this.addHeader(headers[0])) {
    //     throw new Error('Some headers are invalid');
    //   } else {
    //     return true;
    //   }
    // }
    const normalizedHeaders = headers.map((h) => utils.normalizeHeader(h));
    const tip = this.getTipHeader();
    // Handle 1 block intersection of batches
    if (tip.hash === normalizedHeaders[0].hash) {
      normalizedHeaders.splice(0, 1);
    }

    if (normalizedHeaders.length === 0) {
      // The batch already in the chain, do nothing
      return [];
    }

    const isOrphan = !SpvChain.isParentChild(normalizedHeaders[0], tip);

    const allValid = normalizedHeaders.reduce(
      (acc, header, index, array) => {
        const previousHeaders = normalizedHeaders.slice(0, index);
        if (index !== 0) {
          if (!SpvChain.isParentChild(header, array[index - 1])) {
            throw new Error(`SPV: Header ${header.hash} is not a child of ${array[index - 1].hash}`);
          }

          if (!this.isValid(header, previousHeaders)) {
            throw new Error(`SPV: Header ${header.hash} is invalid`);
          }
          return acc && true;
        }
        if (isOrphan) {
          if (!this.isValid(header, previousHeaders)) {
            throw new Error('Some headers are invalid');
          }
          return acc && true;
        }
        if (!this.isValid(header, this.getLongestChain())) {
          throw new Error('Some headers are invalid');
        }
        return acc && true;
      }, true,
    );
    if (!allValid) {
      throw new Error('Some headers are invalid');
    }
    if (isOrphan) {
      normalizedHeaders.forEach((header) => {
        this.orphansHashes.add(header.hash);
      });
      this.orphanChunks.push(normalizedHeaders);
    } else {
      this.appendHeadersToLongestChain(normalizedHeaders);
    }
    if (this.orphanChunks.length > 0) {
      this.orphanChunksReconnect();
    }
    this.checkPruneBlocks();
    return normalizedHeaders;
  }
};

module.exports = SpvChain;
