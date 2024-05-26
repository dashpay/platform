const EventEmitter = require('events');
const { SimplifiedMNListDiff } = require('@dashevo/dashcore-lib');
const logger = require('./logger');

const NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

class MasternodeListSync extends EventEmitter {
  /**
   * @type {SimplifiedMNListDiff}
   */
  fullList;

  /**
   * @type {string}
   */
  previousBlockHash;

  /**
   * @type {CoreRpcClient}
   */
  coreRpc;

  /**
   * @type {string}
   */
  network;

  /**
   * @param {CoreRpcClient} coreRpc
   * @param {string} network
   */
  constructor(coreRpc, network) {
    super();

    this.coreRpc = coreRpc;
    this.network = network;

    this.setMaxListeners(1000);
  }

  /**
   * @param {string} blockHash
   * @return {Promise<void>}
   */
  async sync(blockHash) {
    const fullDiffObject = await this.coreRpc.getMnListDiff(NULL_HASH, blockHash);

    this.fullList = new SimplifiedMNListDiff(fullDiffObject, this.network);

    const latestDiffObject = await this.coreRpc.getMnListDiff(this.previousBlockHash, blockHash);
    const latestDiff = new SimplifiedMNListDiff(latestDiffObject, this.network);

    this.emit(MasternodeListSync.EVENT_DIFF, latestDiff);

    logger.debug(`Masternode list updated to ${blockHash}`);
  }

  /**
   * Sync to the tip
   *
   * @return {Promise<void>}
   */
  async syncToBestBlock() {
    const blockHash = await this.coreRpc.getBestBlockHash();
    await this.sync(blockHash);
  }

  /**
   * @return {SimplifiedMNListDiff}
   */
  getFullList() {
    return this.fullList;
  }
}

MasternodeListSync.EVENT_DIFF = 'diff';

module.exports = MasternodeListSync;
