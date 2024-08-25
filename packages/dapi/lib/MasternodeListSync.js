const EventEmitter = require('events');
const cbor = require('cbor');
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

    const previousBlockHash = this.blockHash;
    const previousBlockHeight = this.blockHeight;

    // TODO: We can't use dashcore-lib SimplifiedMNListDiff toBuffer method, because due to SML
    //  design it's impossible to deserialize it back without knowing of the protocol version.
    //  In future, we want to switch to Rust implementation of SML so we don't want to spend
    //  time on fixing this issue in JS dashcore-lib
    this.fullDiffBuffer = await cbor.encodeAsync(fullDiffObject);
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

      // TODO: We can't use dashcore-lib SimplifiedMNListDiff toBuffer method, because due to SML
      //  design it's impossible to deserialize it back without knowing of the protocol version.
      //  In future, we want to switch to Rust implementation of SML so we don't want to spend
      //  time on fixing this issue in JS dashcore-lib
      const diffBuffer = await cbor.encodeAsync(diffObject);

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
