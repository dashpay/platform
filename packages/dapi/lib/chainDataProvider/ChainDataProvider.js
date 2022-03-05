const { EventEmitter } = require('events');
const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');
const log = require('../log');

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
  }

  /**
   * @private
   * @param blockHash {Buffer}
   */
  blockHashHandler(blockHash) {
    this.emit(this.events.NEW_BLOCK_HEADER, blockHash.toString('hex'));
  }

  /**
   * @private
   * @param rawChainLock {Buffer}
   */
  chainLockHandler(rawChainLock) {
    const chainLock = new ChainLock(rawChainLock);

    this.chainLock = chainLock;

    this.emit(this.events.NEW_CHAIN_LOCK, chainLock);
  }

  /**
   * Grabs most recent chainlock
   * @returns {Promise<void>}
   */
  async init() {
    try {
      const data = await this.coreRpcAPI.getBestChainLock();
      const chainLock = new ChainLock(data);

      this.chainLockHandler(chainLock.toBuffer());
    } catch (e) {
      if (e.code === -32603) {
        log.info('No chain lock available in dashcore node');
      } else {
        throw e;
      }
    }

    this.zmqClient.on(this.zmqClient.topics.rawchainlock,
      (buffer) => this.chainLockHandler(buffer));
    this.zmqClient.on(this.zmqClient.topics.hashblock,
      (buffer) => this.blockHashHandler(buffer));
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
    const cached = this.blockHeadersCache.get(blockHash);

    if (cached) {
      return cached;
    }

    const rawBlockHeader = await this.coreRpcAPI.getBlockHeader(blockHash);
    const blockHeaderBuffer = Buffer.from(rawBlockHeader, 'hex');
    const blockHeader = new BlockHeader(blockHeaderBuffer);

    this.blockHeadersCache.set(blockHash, blockHeader);

    return new BlockHeader(blockHeaderBuffer);
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
    let fetchCount = count;

    const blockHeights = Array.from({ length: count })
      .map((e, i) => fromHeight + i);

    const cachedBlockHeaders = blockHeights
      .map((blockHeight) => this.blockHeadersCache.get(blockHeight));
    const [firstCachedItem] = cachedBlockHeaders;

    let lastCachedIndex = -1;

    if (firstCachedItem) {
      const firstMissingIndex = cachedBlockHeaders.indexOf(undefined);

      if (firstMissingIndex !== -1) {
        lastCachedIndex = firstMissingIndex - 1;

        const blockHeader = cachedBlockHeaders[lastCachedIndex];

        startHash = blockHeader.hash;
        fetchCount -= lastCachedIndex;
      } else {
        // return cache if we do not miss anything
        return cachedBlockHeaders;
      }
    }

    const missingBlockHeaders = await this.coreRpcAPI.getBlockHeaders(startHash, fetchCount);
    const rawBlockHeaders = [...((cachedBlockHeaders.slice(0,
      lastCachedIndex !== -1 ? lastCachedIndex : 0)).map((e) => e.toString('hex'))), ...missingBlockHeaders];

    missingBlockHeaders.forEach((e, i) => this.blockHeadersCache.set(fromHeight + i, new BlockHeader(Buffer.from(e, 'hex'))));

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

ChainDataProvider.prototype.events = {
  NEW_BLOCK_HEADER: 'NEW_BLOCK_HEADER',
  NEW_CHAIN_LOCK: 'NEW_CHAIN_LOCK',
};

module.exports = ChainDataProvider;
