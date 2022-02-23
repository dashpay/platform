const { EventEmitter } = require('events');
const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');
const blockHeadersCache = require('./blockheaders-cache');

class ChainDataProvider extends EventEmitter {
  constructor(coreRpcClient, zmqClient) {
    super();

    this.coreRpcAPI = coreRpcClient;
    this.zmqClient = zmqClient;

    this.chainLock = null;
  }

  blockHashHandler(blockHash) {
    this.emit(this.events.NEW_BLOCK_HEADER, blockHash);
  }

  chainLockHandler(rawChainLock) {
    this.chainLock = new ChainLock(Buffer.from(rawChainLock));

    this.emit(this.events.NEW_CHAIN_LOCK);
  }

  async init() {
    const chainLock = await this.coreRpcAPI.getBestChainLock();
    this.chainLock = new ChainLock(chainLock);

    this.zmqClient.on(this.zmqClient.topics.rawtx, this.chainLockHandler);
    this.zmqClient.on(this.zmqClient.topics.rawblock, this.blockHashHandler);
  }

  async getBlockHeader(hash) {
    const cached = blockHeadersCache.get(hash);

    if (cached) {
      return new BlockHeader(Buffer.from(cached, 'hex'));
    }

    const rawBlockHeader = await this.coreRpcAPI.getBlockHeader(hash);

    blockHeadersCache.set(hash, rawBlockHeader);

    return new BlockHeader(Buffer.from(rawBlockHeader, 'hex'));
  }

  async getBlockHeaders(fromHash, count) {
    let startHash = fromHash;
    let fetchCount = count;

      const { height } = await this.coreRpcAPI.getBlockStats(fromHash, ['height']);

    const blockHeights = [...Array(count).keys()]
      .map((e, i) => height + i);

    const cachedBlockHeaders = blockHeights
      .map((blockHeight) => blockHeadersCache.get(blockHeight));
    const [firstCachedItem] = cachedBlockHeaders;

    let lastCachedIndex = -1;

    if (firstCachedItem) {
      const firstMissingIndex = cachedBlockHeaders.indexOf(undefined);

      // return cache if we do not miss anything
      if (cachedBlockHeaders.filter((e) => !!e).length === count) {
        return cachedBlockHeaders.map((e) =>  new BlockHeader(Buffer.from(e, 'hex')));
      }

      if (firstMissingIndex !== -1) {
        lastCachedIndex = firstMissingIndex - 1;

        const rawBlockHeader = cachedBlockHeaders[lastCachedIndex];
        const blockHeader = new BlockHeader(Buffer.from(rawBlockHeader, 'hex'));

        startHash = blockHeader.hash.toString('hex');
        fetchCount -= lastCachedIndex;
      }
    }

    const missingBlockHeaders = await this.coreRpcAPI.getBlockHeaders(startHash, fetchCount);
    const rawBlockHeaders = [...cachedBlockHeaders.slice(0,
      lastCachedIndex !== -1 ? lastCachedIndex : 0), ...missingBlockHeaders];

    missingBlockHeaders.forEach((e, i) => blockHeadersCache.set(height + i, e));

    return rawBlockHeaders.map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));
  }

  /**
   *
   * @returns {Promise<ChainLock>}
   */
  async getBestChainLock() {
    return this.chainLock;
  }
}

ChainDataProvider.prototype.events = {
  NEW_BLOCK_HEADER: 'NEW_BLOCK_HEADER',
  NEW_CHAIN_LOCK: 'NEW_CHAIN_LOCK',
};

module.exports = ChainDataProvider;
