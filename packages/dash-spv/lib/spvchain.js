const X11 = require('./x11');

const config = require('../config/config');
const Consensus = require('./consensus');
const utils = require('./utils');
const SPVError = require('./errors/SPVError');

const CHAIN_TYPE_NETWORKS = {
  testnet: 'testnet',
  devnet: 'devnet',
  local: 'regtest',
  regtest: 'regtest',
  lowdiff: 'regtest',
  livenet: 'mainnet',
  mainnet: 'mainnet',
};

const createGenesis = (chainType) => {
  switch (chainType) {
    case 'testnet': return config.getTestnetGenesis();
    case 'devnet': return config.getDevnetGenesis();
    case 'local':
    case 'regtest': return config.getRegtestGenesis();
    case 'livenet':
    case 'mainnet': return config.getLivenetGenesis();
    case 'lowdiff': return config.getLowDiffGenesis();
    default: throw new SPVError(`Unsupported chain type ${chainType}`);
  }
};

const SpvChain = class {
  constructor(chainType, confirms = 100) {
    this.confirmsBeforeFinal = confirms;
    this.network = CHAIN_TYPE_NETWORKS[chainType];
    if (!this.network) {
      throw new SPVError(`Unsupported chain type "${chainType}"`);
    }
    this.genesis = createGenesis(chainType);

    this.reset();
  }

  static async wasmX11Ready() {
    return X11.load();
  }

  reset() {
    this.allBranches = [[]];
    this.orphanChunks = [];
    this.prunedHeaders = [];

    this.hashesByHeight = new Map();
    this.heightByHash = new Map();
    this.orphansHashes = new Set();
    // Height of the first header in the chain.
    // 0 - if we are synchronizing from genesis
    // > 0 if part of the chain is pruned and we are synchronizing from checkpoint where
    this.startBlockHeight = null;

    // Head of pending first header.
    // Used in situations where we don't know root in advance
    this.pendingStartBlockHeight = null;
  }

  /**
   * Initializes chain with a start block and height
   * - When no arguments provided, chain is initialized from genesis block
   * - If startBlock is provided, height must be provided as well
   * @param {BlockHeader} [startBlock]
   * @param {number} [height]
   */
  initialize(startBlock, height) {
    if (!X11.ready()) {
      throw new SPVError('X11 wasm not ready, call wasmX11Ready() first');
    }

    if (this.startBlockHeight !== null) {
      throw new SPVError('Chain already initialized');
    }

    if (startBlock && typeof height === 'number') {
      // eslint-disable-next-line
      startBlock = utils.normalizeHeader(startBlock);
    } else if (!startBlock && typeof height !== 'number') {
      // eslint-disable-next-line
      startBlock = this.genesis;
      // eslint-disable-next-line
      height = 0;
    } else {
      throw new SPVError('Initialization error, please provide both startBlock and height');
    }

    this.startBlockHeight = height;
    this.hashesByHeight = new Map([[height, startBlock.hash]]);
    this.heightByHash = new Map([[startBlock.hash, height]]);
    this.setAllBranches(startBlock);
  }

  initialized() {
    return this.startBlockHeight !== null || this.pendingStartBlockHeight !== null;
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
  setAllBranches(node, branch = []) {
    if (!node) {
      throw new SPVError('Root node for a branch is not defined');
    }

    this.allBranches = [];
    branch.push(node);

    this.allBranches.push(branch);
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
  getOrphanChunks() {
    return this.orphanChunks;
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

    return true;
  }

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
   * adds an array of valid headers to the longest spv chain.
   * If they cannot be connected to last tip they get temporarily
   * added to an orphan array for possible later reconnection
   *
   * @param {Object[]|string[]|buffer[]} headers
   * @param {number} batchHeadHeight - height of the first header in the array
   * @return {BlockHeader[]}
   */
  addHeaders(headers, batchHeadHeight = 0) {
    if (!X11.ready()) {
      throw new SPVError('X11 wasm not ready, call wasmX11Ready() first');
    }

    if (!this.initialized()) {
      throw new SPVError('Chain not initialized, either call initialize() or set pendingStartBlockHeight');
    }

    const normalizedHeaders = headers.map((h) => utils.normalizeHeader(h));

    let isOrphan = false;

    // Chain is initialized, root block, and it's height are known
    if (this.pendingStartBlockHeight === null) {
      const tip = this.getTipHeader();
      // Handle 1 block intersection of batches
      if (tip.hash === normalizedHeaders[0].hash) {
        normalizedHeaders.splice(0, 1);
      }

      if (normalizedHeaders.length === 0) {
        // The batch already in the chain, do nothing
        return [];
      }

      const firstHeader = normalizedHeaders[0];

      isOrphan = !SpvChain.isParentChild(firstHeader, tip);
    } else if (batchHeadHeight === this.pendingStartBlockHeight) {
      // Header at pendingStartBlockHeight is found, initialize chain
      this.startBlockHeight = this.pendingStartBlockHeight;
      this.pendingStartBlockHeight = null;
      isOrphan = false;
    } else if (batchHeadHeight > this.pendingStartBlockHeight) {
      // Orphan chunk has arrived
      isOrphan = true;
    } else {
      throw new SPVError(`Batch at invalid height arrived: ${batchHeadHeight}, expected > ${this.pendingStartBlockHeight}`);
    }

    const allValid = normalizedHeaders.reduce((acc, header, index, array) => {
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
    }, true);
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
