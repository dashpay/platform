const { EventEmitter } = require('events');
const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');
const log = require('../log');

class ChainDataProvider extends EventEmitter {
  constructor(coreRpcClient, zmqClient, blockHeadersCache) {
    super();

    this.coreRpcAPI = coreRpcClient;
    this.zmqClient = zmqClient;
    this.blockHeadersCache = blockHeadersCache;

    this.chainLock = null;
  }

  /**
   * @private
   * @param blockHash {string}
   */
  blockHashHandler(blockHash) {
    this.emit(this.events.NEW_BLOCK_HEADER, blockHash);
  }

  /**
   * @private
   * @param rawChainLock {string}
   */
  chainLockHandler(rawChainLock) {
    this.chainLock = new ChainLock(Buffer.from(rawChainLock, 'hex'));

    this.emit(this.events.NEW_CHAIN_LOCK);
  }

  async init() {
    let chainLock;

    try {
      chainLock = await this.coreRpcAPI.getBestChainLock();
    } catch (e) {
      if (e.code === -32603) {
        log.info('No chain lock available in dashcore node');
      } else {
        throw e;
      }
    }

    this.chainLock = new ChainLock(chainLock);

    this.zmqClient.on(this.zmqClient.topics.rawtx, this.chainLockHandler);
    this.zmqClient.on(this.zmqClient.topics.rawblock, this.blockHashHandler);
  }

  async getBlockHash(height) {
    return this.coreRpcAPI.getBlockHash(height);
  }

  async getBlockHeader(hash) {
    const cached = this.blockHeadersCache.get(hash);

    if (cached) {
      return new BlockHeader(cached);
    }

    const rawBlockHeader = await this.coreRpcAPI.getBlockHeader(hash);
    const blockHeaderBuffer = Buffer.from(rawBlockHeader, 'hex');

    this.blockHeadersCache.set(hash, blockHeaderBuffer);

    return new BlockHeader(blockHeaderBuffer);
  }

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

      // return cache if we do not miss anything
      if (cachedBlockHeaders.filter((e) => !!e).length === count) {
        return cachedBlockHeaders.map((e) => new BlockHeader(e));
      }

      if (firstMissingIndex !== -1) {
        lastCachedIndex = firstMissingIndex - 1;

        const rawBlockHeader = cachedBlockHeaders[lastCachedIndex];
        const blockHeader = new BlockHeader(rawBlockHeader);

        startHash = blockHeader.hash;
        fetchCount -= lastCachedIndex;
      }
    }

    const missingBlockHeaders = await this.coreRpcAPI.getBlockHeaders(startHash, fetchCount);
    const rawBlockHeaders = [...((cachedBlockHeaders.slice(0,
      lastCachedIndex !== -1 ? lastCachedIndex : 0)).map((e) => e.toString('hex'))), ...missingBlockHeaders];

    missingBlockHeaders.forEach((e, i) => this.blockHeadersCache.set(fromHeight + i, Buffer.from(e, 'hex')));

    return rawBlockHeaders.map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));
  }

  /**
   *
   * @returns {ChainLock}
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
