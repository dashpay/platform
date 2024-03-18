const ChainLockSigMessage = require('@dashevo/dashcore-lib/lib/zmqMessages/ChainLockSigMessage');
const { EventEmitter } = require('events');
const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');
const logger = require('../logger');

const REORG_SAFE_DEPTH = 6;

/**
 * Data access layer with caching support
 */
class ChainDataProvider extends EventEmitter {
  /**
   *
   * @param coreRpcClient {CoreRpcClient}
   * @param zmqClient {ZmqClient}
   * @param blockHeadersCache {BlockHeadersCache}
   */
  constructor(coreRpcClient, zmqClient, blockHeadersCache) {
    super();

    this.coreRpcAPI = coreRpcClient;
    this.zmqClient = zmqClient;
    this.blockHeadersCache = blockHeadersCache;

    this.chainLock = null;
    this.chainHeight = -1;
  }

  /**
   * @private
   * @param blockHash {Buffer}
   */
  blockHashHandler(blockHash) {
    this.emit(this.events.NEW_BLOCK_HEADER, blockHash.toString('hex'));
    this.coreRpcAPI.getBestBlockHeight()
      .then((height) => {
        this.chainHeight = height;
      })
      .catch((e) => this.emit('error', e));
  }

  /**
   * @private
   * @param rawChainLock {Object|ChainLock} JSON-object from getBestChainLock or ChainLock instance
   */
  chainLockHandler(rawChainLock) {
    const chainLock = new ChainLock(rawChainLock);

    this.chainLock = chainLock;

    this.emit(this.events.NEW_CHAIN_LOCK, chainLock);
  }

  /**
   *
   * @param {Buffer} rawChainLockSigBuffer
   */
  rawChainLockSigHandler(rawChainLockSigBuffer) {
    try {
      const { chainLock } = new ChainLockSigMessage(rawChainLockSigBuffer);

      this.chainLockHandler(chainLock);
    } catch (e) {
      // eslint-disable no-empty
    }
  }

  /**
   * Grabs most recent chainlock
   * @returns {Promise<void>}
   */
  async init() {
    this.chainHeight = await this.coreRpcAPI.getBestBlockHeight();

    try {
      const chainLock = await this.coreRpcAPI.getBestChainLock();

      this.chainLockHandler(chainLock);
    } catch (e) {
      if (e.code === -32603) {
        logger.info('No chain lock available in dashcore node');
      } else {
        throw e;
      }
    }

    this.zmqClient.on(
      this.zmqClient.topics.rawchainlocksig,
      (buffer) => this.rawChainLockSigHandler(buffer),
    );
    this.zmqClient.on(
      this.zmqClient.topics.hashblock,
      (buffer) => this.blockHashHandler(buffer),
    );
  }

  /**
   * Get block hash by height
   * @param height {number}
   * @returns {Promise<string>}
   */
  async getBlockHash(height) {
    return this.coreRpcAPI.getBlockHash(height);
  }

  /**
   * Get block header by block hash
   * @param blockHash {string}
   * @returns {Promise<BlockHeader>}
   */
  async getBlockHeader(blockHash) {
    // Check if we already have header in cache
    const cached = this.blockHeadersCache.get(blockHash);

    if (cached) {
      return cached;
    }

    const rawBlockHeader = await this.coreRpcAPI.getBlockHeader(blockHash);
    const blockHeaderBuffer = Buffer.from(rawBlockHeader, 'hex');
    const blockHeader = new BlockHeader(blockHeaderBuffer);

    // Put header into cache
    this.blockHeadersCache.set(blockHash, blockHeader);

    return blockHeader;
  }

  /**
   * Receive set of block headers with cache support
   * @param fromHash {string}
   * @param fromHeight {number}
   * @param count {number}
   * @returns {Promise<BlockHeader[]>}
   */
  async getBlockHeaders(fromHash, fromHeight, count) {
    let startHash = fromHash;
    let startHeight = fromHeight;
    let fetchCount = count;

    // TODO: optimize this logic with one for loop
    // of range [startHeight...startHeight + fetchCount - 1]
    // instead of producing intermediary arrays with heights and cached values

    // Calculate heights for every header in the batch
    const blockHeights = Array.from({ length: count })
      .map((e, i) => startHeight + i);

    // Obtain headers from cache. If there's no headers in cache
    // array will be filled with undefined values
    const cachedBlockHeaders = blockHeights
      .map((blockHeight) => this.blockHeadersCache.get(blockHeight));
    const [firstCachedItem] = cachedBlockHeaders;

    let lastCachedIndex = -1;

    // If we have first item in cache, then proceed finding the rest ones
    // otherwise, re-fetch all headers from dashcore
    if (firstCachedItem) {
      // Find index of the item that follows last cached item
      const firstMissingIndex = cachedBlockHeaders.indexOf(undefined);

      // If we don't have some items in cache, then we need to fetch
      // Otherwise the cache is considered complete, and we return values from it
      if (firstMissingIndex !== -1) {
        lastCachedIndex = firstMissingIndex - 1;

        const blockHeader = cachedBlockHeaders[lastCachedIndex];

        // Update startHash, startHeight and fetchCount in order
        // to fetch from dashcore only missing items
        startHash = blockHeader.hash;
        startHeight += lastCachedIndex;
        fetchCount -= lastCachedIndex;
      } else {
        // return cache if we do not miss anything
        return cachedBlockHeaders;
      }
    }

    // Fetch missing items
    const missingBlockHeaders = await this.coreRpcAPI.getBlockHeaders(startHash, fetchCount);
    // Concatenate all items together
    const rawBlockHeaders = [...((cachedBlockHeaders.slice(
      0,
      lastCachedIndex !== -1 ? lastCachedIndex : 0,
    )).map((e) => e.toString('hex'))), ...missingBlockHeaders];

    // Calculate safe height in order to cache headers that are
    // not subjected to reorgs

    // Ignore last 6 headers by default
    let safeCacheHeight = this.chainHeight - REORG_SAFE_DEPTH;

    // In case we have a chain lock with the higher value, use it
    if (this.chainLock && this.chainLock.height > safeCacheHeight) {
      safeCacheHeight = this.chainLock.height;
    }

    // Put missing items in cache
    missingBlockHeaders.forEach((e, i) => {
      const headerHeight = startHeight + i;
      if (headerHeight <= safeCacheHeight) {
        const header = new BlockHeader(Buffer.from(e, 'hex'));
        this.blockHeadersCache.set(headerHeight, header);
      }
    });

    return rawBlockHeaders.map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));
  }

  /**
   * Return best chain lock
   * @returns {ChainLock|null}
   */
  getBestChainLock() {
    return this.chainLock;
  }
}

ChainDataProvider.REORG_SAFE_DEPTH = REORG_SAFE_DEPTH;

ChainDataProvider.prototype.events = {
  NEW_BLOCK_HEADER: 'NEW_BLOCK_HEADER',
  NEW_CHAIN_LOCK: 'NEW_CHAIN_LOCK',
};

module.exports = ChainDataProvider;
