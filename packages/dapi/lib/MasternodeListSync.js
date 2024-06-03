const EventEmitter = require('events');
const { SimplifiedMNListDiff } = require('@dashevo/dashcore-lib');
const logger = require('./logger');

const NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

/**
 * @param {ChainLock} chainLock
 * @return {string}
 */
function chainLockToBlockHashHex(chainLock) {
  return chainLock.blockHash.toString('hex');
}

class MasternodeListSync extends EventEmitter {
  /**
   * @type {Buffer}
   */
  fullDiffBuffer;

  /**
   * @type {number}
   */
  blockHeight = 0;

  /**
   * @type {string}
   */
  blockHash;

  /**
   * @type {CoreRpcClient}
   */
  coreRpc;

  /**
   * @type {ChainDataProvider}
   */
  chainDataProvider;

  /**
   * @type {string}
   */
  network;

  /**
   * @param {CoreRpcClient} coreRpc
   * @param {ChainDataProvider} chainDataProvider
   * @param {string} network
   */
  constructor(coreRpc, chainDataProvider, network) {
    super();

    this.coreRpc = coreRpc;
    this.chainDataProvider = chainDataProvider;
    this.network = network;
    this.logger = logger.child({
      service: 'MasternodeListSync',
    });

    this.setMaxListeners(1000);
  }

  /**
   * @param {string} blockHash
   * @param {number} blockHeight
   * @return {Promise<void>}
   */
  async sync(blockHash, blockHeight) {
    const fullDiffObject = await this.coreRpc.getMnListDiff(NULL_HASH, blockHash);

    // TODO: It's a dirty hack to fix serialisation issue, introduced by reverting version of the
    //  diff from 2 to 1. So now version 1 of diff contains entries of version 1 and 2 and
    //  we don't know how to parse it since version field is introduced in version 2.
    fullDiffObject.nVersion = 2;

    const previousBlockHash = this.blockHash;
    const previousBlockHeight = this.blockHeight;

    const fullDiff = new SimplifiedMNListDiff(fullDiffObject, this.network);

    this.fullDiffBuffer = fullDiff.toBuffer();
    this.blockHeight = blockHeight;
    this.blockHash = blockHash;

    this.logger.debug(
      {
        blockHash,
        blockHeight,
        network: this.network,
      },
      `Full masternode list updated to block ${blockHeight}`,
    );

    if (previousBlockHash) {
      const diffObject = await this.coreRpc.getMnListDiff(previousBlockHash, blockHash);

      // TODO: It's a dirty hack to fix serialisation issue, introduced by reverting version of the
      //  diff from 2 to 1. So now version 1 of diff contains entries of version 1 and 2 and we
      //  don't know how to parse it since version field is introduced in version 2.
      diffObject.nVersion = 2;

      const diff = new SimplifiedMNListDiff(diffObject, this.network);
      const diffBuffer = diff.toBuffer();

      this.logger.debug({
        previousBlockHash,
        blockHash,
        previousBlockHeight,
        blockHeight,
        network: this.network,
      }, `New diff from block ${previousBlockHeight} to ${blockHeight} received`);

      this.emit(MasternodeListSync.EVENT_DIFF, diffBuffer, blockHeight, blockHash);
    }

    this.blockHash = blockHash;
  }

  /**
   * @return {Promise<void>}
   */
  async init() {
    // Init makes sure, that we have full diff, so we need to use the existing best chain lock
    // or wait for the first one

    let resolved = false;

    return new Promise((resolve, reject) => {
      const bestChainLock = this.chainDataProvider.getBestChainLock();

      this.chainDataProvider.on(this.chainDataProvider.events.NEW_CHAIN_LOCK, (chainLock) => {
        const blockHash = chainLockToBlockHashHex(chainLock);

        this.sync(blockHash, chainLock.height).then(() => {
          // Resolve the promise when chain lock is arrive we don't have any yet
          if (!bestChainLock && !resolved) {
            resolve();
            resolved = true;
          }
        }).catch((error) => {
          this.logger.error({ err: error }, `Failed to sync masternode list: ${error.message}`);

          if (!resolved) {
            reject(error);
            resolved = true;
          }
        });
      });

      if (bestChainLock) {
        const bestBlockHash = chainLockToBlockHashHex(bestChainLock);

        // Resolve promise when we have the best chain lock
        this.sync(bestBlockHash, bestChainLock.height).then(() => {
          if (!resolved) {
            resolve();
            resolved = true;
          }
        }).catch((error) => {
          this.logger.error({ err: error }, `Failed to sync masternode list: ${error.message}`);

          if (!resolved) {
            reject(error);
            resolved = true;
          }
        });
      }
    });
  }

  /**
   * @return {Buffer}
   */
  getFullDiffBuffer() {
    return this.fullDiffBuffer;
  }

  /**
   * @return {number}
   */
  getBlockHeight() {
    return this.blockHeight;
  }

  /**
   * @return {string}
   */
  getBlockHash() {
    return this.blockHash;
  }
}

MasternodeListSync.EVENT_DIFF = 'diff';

module.exports = MasternodeListSync;
