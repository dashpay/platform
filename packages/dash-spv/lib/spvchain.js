const EventEmitter = require('events');
const X11 = require('./x11');

const config = require('../config/config');
const Consensus = require('./consensus');
const pow = require('./pow');
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

// Limits for headers that don't extend the main chain
const MAX_FORKS = 10;

const EVENTS = {
  // (headers, firstHeight): headers that became the end of the main chain,
  // from firstHeight on. After a reorg firstHeight is at or below the old tip.
  CONNECTED: 'connected',
};

class SpvChain extends EventEmitter {
  /**
   * @param {string} chainType
   * @param {number} [confirms] - how many of the last headers can still be
   *   replaced by a fork with more work
   * @param {object} [devnetOptions] - settings a devnet was started with
   * @param {number} [devnetOptions.powTargetSpacing] - Core's -powtargetspacing
   * @param {number} [devnetOptions.minimumDifficultyBlocks] - Core's -minimumdifficultyblocks
   */
  constructor(chainType, confirms = 100, devnetOptions = undefined) {
    super();
    this.confirmsBeforeFinal = confirms;
    this.network = CHAIN_TYPE_NETWORKS[chainType];
    if (!this.network) {
      throw new SPVError(`Unsupported chain type "${chainType}"`);
    }
    // What consensus validation gets: the network, or a devnet with its settings
    this.consensusNetwork = devnetOptions ? { ...devnetOptions, network: this.network } : this.network;
    pow.getNetworkParams(this.consensusNetwork);
    this.genesis = createGenesis(chainType);

    this.reset();
  }

  static async wasmX11Ready() {
    return X11.load();
  }

  reset() {
    // The main chain's unconfirmed headers first, then forks off them
    this.allBranches = [[]];
    this.orphanChunks = [];
    // Confirmed headers kept for validation; older ones are dropped
    this.prunedHeaders = [];
    this.droppedHeadersCount = 0;

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
      startBlock = utils.copyHeader(startBlock);
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

    // Forks off confirmed headers can't win any more
    const firstUnconfirmedHeight = this.getFirstUnconfirmedHeight();
    this.allBranches = [longestChain, ...this.normalizeForks(this.getForks()).filter(
      (fork) => this.heightByHash.get(SpvChain.getParentHash(fork)) >= firstUnconfirmedHeight - 1,
    )];

    // Keep only the confirmed headers validation can read
    const dropCount = this.prunedHeaders.length - pow.MAX_HISTORY;
    if (dropCount > 0) {
      this.prunedHeaders.splice(0, dropCount).forEach((header) => {
        this.hashesByHeight.delete(this.heightByHash.get(header.hash));
        this.heightByHash.delete(header.hash);
      });
      this.droppedHeadersCount += dropCount;
    }
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

  /**
   * @private
   * Validates headers that continue the main chain's tip and appends them
   * @param {BlockHeader[]} headers
   */
  extendLongestChain(headers) {
    const firstHeight = this.getTipHeight() + 1;
    this.validateConnectedHeaders(headers, this.getValidationHistory(), firstHeight);

    headers.forEach((header, i) => {
      const height = firstHeight + i;
      this.hashesByHeight.set(height, header.hash);
      this.heightByHash.set(header.hash, height);
    });

    this.allBranches[0] = this.getLongestChain().concat(headers);

    this.emit(EVENTS.CONNECTED, headers, firstHeight);
  }

  /** @private */
  getAllBranches() {
    return this.allBranches;
  }

  /** @private */
  getForks() {
    return this.allBranches.slice(1);
  }

  /**
   * @private
   * Each fork as the headers it has off the main chain, rooted at a main-chain
   * header: drops a start that became part of the main chain, and roots a fork
   * off another fork (such as the headers a reorg replaced) at that fork's root
   * @param {BlockHeader[][]} forks
   * @return {BlockHeader[][]}
   */
  normalizeForks(forks) {
    const isOnMainChain = (header) => this.heightByHash.has(header.hash);
    let pending = forks
      .map((fork) => fork.slice(fork.findIndex((header) => !isOnMainChain(header))))
      .filter((fork) => fork.length && !isOnMainChain(fork[0]));
    const normalized = [];

    // A fork off another fork is rooted once that fork is
    const root = (fork) => {
      const parentHash = SpvChain.getParentHash(fork);
      if (this.heightByHash.has(parentHash)) {
        return fork;
      }
      const base = normalized.find((other) => other.some((header) => header.hash === parentHash));
      return base
        ? base.slice(0, base.findIndex((header) => header.hash === parentHash) + 1).concat(fork)
        : null;
    };
    let previousCount;
    do {
      previousCount = pending.length;
      const unresolved = [];
      pending.forEach((fork) => {
        const rooted = root(fork);
        if (rooted) {
          normalized.push(rooted);
        } else {
          unresolved.push(fork);
        }
      });
      pending = unresolved;
    } while (pending.length && pending.length < previousCount);

    // Drop forks another one already contains
    return normalized.filter((fork, i) => !normalized.some((other, j) => j !== i
      && other.length >= fork.length
      && (other.length > fork.length || j < i)
      && fork.every((header, k) => other[k].hash === header.hash)));
  }

  /** @private */
  static getParentHash(headers) {
    return utils.getCorrectedHash(headers[0].prevHash);
  }

  /** @private */
  isDuplicate(hash) {
    return this.hasHeader(hash) || this.orphansHashes.has(hash);
  }

  /**
   * @private
   * @param {string} hash
   * @return {boolean} whether the main chain or a fork has the header
   */
  hasHeader(hash) {
    return this.heightByHash.has(hash) || this.findInForks(hash) !== null;
  }

  /**
   * @private
   * @param {string} hash
   * @return {{ forkIndex: number, index: number }|null} position in allBranches
   */
  findInForks(hash) {
    for (let forkIndex = 1; forkIndex < this.allBranches.length; forkIndex += 1) {
      const index = this.allBranches[forkIndex].findIndex((header) => header.hash === hash);
      if (index >= 0) {
        return { forkIndex, index };
      }
    }
    return null;
  }

  /**
   * Drops headers that don't connect to anything known yet
   */
  dropOrphans() {
    this.orphanChunks = [];
    this.orphansHashes.clear();
  }

  /**
   * @private
   * Validates headers that continue the chain at `parentHash` and either
   * extends the main chain, starts or extends a fork, or reorgs to a fork
   * that now has more work.
   * @param {BlockHeader[]} headers
   * @return {boolean} false if they connect to nothing known
   */
  connectHeaders(headers) {
    const parentHash = SpvChain.getParentHash(headers);

    if (parentHash === this.getTipHash()) {
      this.extendLongestChain(headers);
      return true;
    }

    const longestChain = this.getLongestChain();
    const firstUnconfirmedHeight = this.getFirstUnconfirmedHeight();
    const inFork = this.findInForks(parentHash);
    let fork;
    if (inFork) {
      const branch = this.allBranches[inFork.forkIndex];
      // Extends the fork, or starts a new one sharing the start of it
      fork = inFork.index === branch.length - 1 ? branch : branch.slice(0, inFork.index + 1);
    } else if (this.heightByHash.has(parentHash)) {
      if (this.heightByHash.get(parentHash) < firstUnconfirmedHeight - 1) {
        throw new SPVError(`SPV: Header ${headers[0].hash} forks off confirmed header ${parentHash}`);
      }
      // A new fork off the main chain
      fork = [];
    } else {
      return false;
    }

    const forkParentHash = fork.length ? SpvChain.getParentHash(fork) : parentHash;
    const forkParentHeight = this.heightByHash.get(forkParentHash);
    this.validateConnectedHeaders(
      headers,
      this.getValidationHistory(forkParentHeight).concat(fork),
      forkParentHeight + fork.length + 1,
    );

    // Like Core, switch only to a fork with strictly more work
    const extended = fork.concat(headers);
    const replaced = longestChain.slice(forkParentHeight - firstUnconfirmedHeight + 1);
    const wins = pow.getChainWork(extended) > pow.getChainWork(replaced);

    // Forks kept on the side are bounded: in length, and in number by
    // dropping the oldest
    if (!wins && extended.length > 2 * this.confirmsBeforeFinal) {
      throw new SPVError(`SPV: Fork at ${headers[0].hash} is longer than ${2 * this.confirmsBeforeFinal} headers without more work`);
    }
    fork.push(...headers);
    if (!this.allBranches.includes(fork)) {
      this.allBranches.push(fork);
    }

    if (wins) {
      // The replaced headers become a fork too
      this.reorganize(this.allBranches.indexOf(fork), forkParentHeight);
    }
    if (this.allBranches.length > MAX_FORKS + 1) {
      this.allBranches.splice(1, this.allBranches.length - MAX_FORKS - 1);
    }

    return true;
  }

  /** @private */
  reorganize(forkIndex, forkParentHeight) {
    const longestChain = this.getLongestChain();
    const fork = this.allBranches[forkIndex];

    const replaced = longestChain.splice(forkParentHeight - this.getFirstUnconfirmedHeight() + 1);
    replaced.forEach((header) => {
      this.hashesByHeight.delete(this.heightByHash.get(header.hash));
      this.heightByHash.delete(header.hash);
    });
    this.allBranches.splice(forkIndex, 1, replaced);

    fork.forEach((header, i) => {
      this.hashesByHeight.set(forkParentHeight + 1 + i, header.hash);
      this.heightByHash.set(header.hash, forkParentHeight + 1 + i);
    });
    longestChain.push(...fork);

    this.allBranches = [longestChain, ...this.normalizeForks(this.getForks())];

    this.emit(EVENTS.CONNECTED, fork, forkParentHeight + 1);
  }

  /** @private */
  orphanChunksReconnect() {
    for (let i = 0; i < this.orphanChunks.length; i += 1) {
      const chunk = this.orphanChunks[i];

      // Its parent joined the main chain or a fork
      if (this.hasHeader(SpvChain.getParentHash(chunk))) {
        chunk.forEach((header) => {
          this.orphansHashes.delete(header.hash);
        });
        this.orphanChunks.splice(i, 1);

        this.connectHeaders(chunk);

        // The chain may now connect a chunk checked earlier
        i = -1;
      }
    }
  }

  /**
   * @private
   * The main chain's headers up to `lastHeight`, as many as difficulty
   * validation reads
   * @param {number} [lastHeight] - the tip by default
   * @return {BlockHeader[]}
   */
  getValidationHistory(lastHeight = this.getTipHeight()) {
    const longestChain = this.getLongestChain();
    const end = longestChain.length - (this.getTipHeight() - lastHeight);
    const fromPruned = pow.MAX_HISTORY - end;
    const prunedTail = fromPruned > 0 ? this.prunedHeaders.slice(-fromPruned) : [];
    return prunedTail.concat(longestChain.slice(0, end)).slice(-pow.MAX_HISTORY);
  }

  /**
   * Height of the main chain's tip
   * @return {number}
   */
  getTipHeight() {
    return this.startBlockHeight + this.droppedHeadersCount + this.prunedHeaders.length
      + this.getLongestChain().length - 1;
  }

  /**
   * @private
   * @return {number} height of the first header in the main chain's unconfirmed part
   */
  getFirstUnconfirmedHeight() {
    return this.getTipHeight() - this.getLongestChain().length + 1;
  }

  /**
   * @private
   * @param {BlockHeader[]} headers
   * @param {BlockHeader[]} initialHistory - headers before them, ending with
   *   the parent of the first one
   * @param {number} firstHeight - height of the first header
   */
  validateConnectedHeaders(headers, initialHistory, firstHeight) {
    const history = initialHistory.slice();

    headers.forEach((header, i) => {
      const previousHeader = history[history.length - 1];
      if (!previousHeader || !SpvChain.isParentChild(header, previousHeader)) {
        throw new SPVError(`SPV: Header ${header.hash} is not connected to verified history`);
      }

      if (this.isDuplicate(header.hash) || !Consensus.isValidBlockHeader(
        header,
        history,
        this.consensusNetwork,
        firstHeight + i,
        this.startBlockHeight,
      )) {
        throw new SPVError(`SPV: Header ${header.hash} is invalid`);
      }

      history.push(header);
      // Trim in bulk so each header costs O(1) on average
      if (history.length > 2 * pow.MAX_HISTORY) {
        history.splice(0, history.length - pow.MAX_HISTORY);
      }
    });
  }

  /** @private */
  getOrphanChunks() {
    return this.orphanChunks;
  }

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
   * Returns the main chain's unconfirmed headers, or with `withPruned` all the
   * headers the chain still keeps (at least the last pow.MAX_HISTORY)
   * @param options
   * @returns {BlockHeader[]}
   */
  getLongestChain(options = { withPruned: false }) {
    const [longestChain] = this.allBranches;

    if (options.withPruned) {
      return this.prunedHeaders.concat(longestChain);
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
    const height = this.heightByHash.get(hash);
    return height === undefined ? undefined : this.getHeaderAt(height);
  }

  /**
   * gets the main chain's header at a height, if the chain still keeps it
   *
   * @param {number} height
   * @return {BlockHeader|undefined}
   */
  getHeaderAt(height) {
    const longestChain = this.getLongestChain();
    const fromTip = this.getTipHeight() - height;
    if (fromTip < 0) {
      return undefined;
    }
    return fromTip < longestChain.length
      ? longestChain[longestChain.length - 1 - fromTip]
      : this.prunedHeaders[this.prunedHeaders.length - 1 - (fromTip - longestChain.length)];
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

    // Copies, so later changes to the caller's objects can't alter the chain
    const normalizedHeaders = headers.map((h) => utils.copyHeader(h));

    let isOrphan = false;

    // Chain is initialized, root block, and it's height are known
    if (this.pendingStartBlockHeight === null) {
      // Skip headers the chain already has, such as an overlap with the last batch
      let knownCount = 0;
      while (knownCount < normalizedHeaders.length
        && this.hasHeader(normalizedHeaders[knownCount].hash)) {
        knownCount += 1;
      }
      normalizedHeaders.splice(0, knownCount);

      if (normalizedHeaders.length === 0) {
        // The batch already in the chain, do nothing
        return [];
      }

      isOrphan = !this.connectHeaders(normalizedHeaders);
    } else if (batchHeadHeight === this.pendingStartBlockHeight) {
      throw new SPVError(
        'SPV chain cannot initialize from an unauthenticated remote checkpoint',
      );
    } else if (batchHeadHeight > this.pendingStartBlockHeight) {
      // Orphan chunk has arrived
      isOrphan = true;
    } else {
      throw new SPVError(`Batch at invalid height arrived: ${batchHeadHeight}, expected > ${this.pendingStartBlockHeight}`);
    }

    if (isOrphan) {
      normalizedHeaders.forEach((header, index) => {
        if (index > 0 && !SpvChain.isParentChild(header, normalizedHeaders[index - 1])) {
          throw new SPVError(
            `SPV: Header ${header.hash} is not a child of ${normalizedHeaders[index - 1].hash}`,
          );
        }
        if (this.isDuplicate(header.hash)
          || !Consensus.isValidBlockHeaderWithoutContext(header, this.consensusNetwork)) {
          throw new SPVError(`SPV: Header ${header.hash} is invalid`);
        }
      });

      normalizedHeaders.forEach((header) => {
        this.orphansHashes.add(header.hash);
      });
      this.orphanChunks.push(normalizedHeaders);
    }
    if (this.orphanChunks.length > 0) {
      this.orphanChunksReconnect();
    }
    this.checkPruneBlocks();
    return normalizedHeaders;
  }
}

SpvChain.EVENTS = EVENTS;

module.exports = SpvChain;
