const config = require('../config/config');
const Consensus = require('./consensus');
const utils = require('./utils');
const SPVError = require('./errors/SPVError');

const SpvChain = class {
  // TODO: move chainType and confirms to `options`
  constructor(chainType, confirms = 100, startBlock, startBlockHeight) {
    this.confirmsBeforeFinal = confirms;

    this.reset(startBlockHeight);
    this.init(chainType, startBlock);

    this.hashesByHeight = new Map([[this.startBlockHeight, this.root.hash]]);

    this.heightByHash = new Map([[this.root.hash, this.startBlockHeight]]);

    // TODO: legacy - remove this
    this.orphanBlocks = [];
  }

  reset(fromBlockHeight = 0) {
    this.root = null;
    this.allBranches = [[]];
    this.orphanChunks = [];
    this.prunedHeaders = [];

    this.hashesByHeight = new Map();
    this.heightByHash = new Map();
    this.orphansHashes = new Set();
    this.startBlockHeight = fromBlockHeight < 0 ? 0 : fromBlockHeight;
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
      case 'local':
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
          throw new SPVError('Unhandled chaintype or startBlock not provided');
        }
        break;
    }
    // this.root.children = [];
    this.setAllBranches();
  }

  validate() {
    const longestChain = this.getLongestChain();
    if (!longestChain.length) {
      throw new SPVError('Empty SPV chain');
    }
    const head = longestChain[longestChain.length - 1];
    const tail = this.prunedHeaders[0] || longestChain[0];

    const headHeight = this.heightByHash.get(head.hash);
    const tailHeight = this.heightByHash.get(tail.hash);

    if (this.orphanChunks.length) {
      throw new SPVError('Chain contains orphan chunks');
    }

    if (typeof headHeight !== 'number') {
      throw new SPVError(`Head header '${head.hash}' height is invalid ${headHeight}`);
    }

    if (typeof tailHeight !== 'number') {
      throw new SPVError(`Tail header '${tail.hash}' height is invalid ${tailHeight}`);
    }

    if (this.hashesByHeight.get(headHeight) !== head.hash) {
      throw new SPVError(`Head header '${head.hash}' height mismatch`);
    }

    if (this.hashesByHeight.get(tailHeight) !== tail.hash) {
      throw new SPVError(`Tail header '${tail.hash}' height mismatch`);
    }

    const expectedChainLength = headHeight - tailHeight + 1;
    const actualChainLength = longestChain.length + this.prunedHeaders.length;

    if (expectedChainLength !== actualChainLength) {
      throw new SPVError(`Chain length mismatch: expected ${expectedChainLength}, actual ${actualChainLength}`);
    }
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
      this.hashesByHeight.set(height, header.hash);
      this.heightByHash.set(header.hash, height);
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
    return this.heightByHash.has(hash) || this.orphansHashes.has(hash);
  }

  /** @private */
  // orphanReconnect() {
  // for (let i = 0; i < this.orphanBlocks.length; i += 1) {
  // const connectionTip = this.findConnection(this.orphanBlocks[i]);
  // if (connectionTip) {
  //   connectionTip.children.push(this.orphanBlocks[i]);
  //   this.orphanBlocks.splice(i, 1);
  // }
  // }
  // }

  /** @private */
  orphanChunksReconnect() {
    // TODO: consider optimizing with map of { [chunkHeadHash]: chunkIndex }
    // to get rid of sorting and make the whole function of O(n) complexity
    this.orphanChunks.sort((a, b) => a[0].timestamp - b[0].timestamp);

    for (let i = 0; i < this.orphanChunks.length; i += 1) {
      const chunk = this.orphanChunks[i];
      const tipHash = this.getTipHash();
      const chunkPrevHash = utils.getCorrectedHash(chunk[0].prevHash);

      if (tipHash === chunkPrevHash) {
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
  // TODO: remove
  // getOrphans() {
  //   return this.orphanBlocks;
  // }

  /** @private */
  getOrphanChunks() {
    return this.orphanChunks;
  }

  /** @private */
  // processValidHeader(header) {
  //   const connection = this.findConnection(header);
  //   if (connection) {
  //     // connection.children.push(header);
  //     this.orphanReconnect();
  //   } else {
  //     this.orphanBlocks.push(header);
  //   }
  // }

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
    const tip = this.getTipHeader();
    return tip && tip.hash;
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
  // addHeader(header) {
  //   const headerNormalised = utils.normalizeHeader(header);
  //
  //   if (this.isValid(headerNormalised, this.getLongestChain())) {
  //     // headerNormalised.children = [];
  //     this.processValidHeader(headerNormalised);
  //     this.setAllBranches();
  //     this.checkPruneBlocks();
  //     return true;
  //   }
  //   return false;
  // }

  /**
   * adds an array of valid headers to the longest spv chain.
   * If they cannot be connected to last tip they get temporarily
   * added to an orphan array for possible later reconnection
   *
   * @param {Object[]|string[]|buffer[]} headers
   * @param {number} batchHeadHeight - height of the first header in the array
   * @return {BlockHeader[]}
   */
  addHeaders(headers, batchHeadHeight = 0) {
    let headHeight = batchHeadHeight;
    const normalizedHeaders = headers.map((h) => utils.normalizeHeader(h));

    const tip = this.getTipHeader();
    // Handle 1 block intersection of batches
    if (tip && tip.hash === normalizedHeaders[0].hash) {
      normalizedHeaders.splice(0, 1);
      // Patch head height value after splice
      headHeight += 1;
    }

    if (normalizedHeaders.length === 0) {
      // The batch already in the chain, do nothing
      return [];
    }

    const firstHeader = normalizedHeaders[0];
    const connectsToTip = tip && SpvChain.isParentChild(firstHeader, tip);

    //
    // Reorg detection
    // Get prev header hash
    const prevHash = utils.getCorrectedHash(firstHeader.prevHash);
    const prevHeaderHeight = this.heightByHash.get(prevHash);

    // Test on initial wallet load
    if (prevHeaderHeight && prevHeaderHeight > 0 && prevHeaderHeight !== headHeight - 1) {
      console.log('SPVCHAIN: Reorg detected.');
      console.log(`------->: Batch head ${firstHeader.hash} at height ${headHeight} has parent at height ${prevHeaderHeight}`);
    }

    const isOrphan = tip ? !connectsToTip
      : headHeight !== this.startBlockHeight;

    const allValid = normalizedHeaders.reduce(
      (acc, header, index, array) => {
        const previousHeaders = normalizedHeaders.slice(0, index);
        if (index !== 0) {
          if (!SpvChain.isParentChild(header, array[index - 1])) {
            throw new SPVError(`SPV: Header ${header.hash} is not a child of ${array[index - 1].hash}`);
          }

          if (!this.isValid(header, previousHeaders)) {
            throw new SPVError(`SPV: Header ${header.hash} is invalid`);
          }
          return acc && true;
        }
        if (isOrphan) {
          if (!this.isValid(header, previousHeaders)) {
            throw new SPVError('Some headers are invalid');
          }
          return acc && true;
        }
        if (!this.isValid(header, this.getLongestChain())) {
          throw new SPVError('Some headers are invalid');
        }
        return acc && true;
      }, true,
    );
    if (!allValid) {
      throw new SPVError('Some headers are invalid');
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
