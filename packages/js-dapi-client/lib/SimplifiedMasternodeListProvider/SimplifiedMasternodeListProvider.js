const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');
const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');

class SimplifiedMasternodeListProvider {
  /**
   *
   * @param {JsonRpcTransport} jsonRpcTransport - JsonRpcTransport instance
   * @param {object} [options] - Options
   * @param {number} [options.updateInterval=60000]
   * @param {string} [options.network]
   */
  constructor(jsonRpcTransport, options = {}) {
    this.jsonRpcTransport = jsonRpcTransport;

    this.options = {
      updateInterval: 60000,
      ...options,
    };

    this.simplifiedMNList = new SimplifiedMNList(undefined, this.options.network);

    this.lastUpdateDate = 0;

    this.baseBlockHash = SimplifiedMasternodeListProvider.NULL_HASH;
  }

  /**
   * Returns simplified masternode list
   *
   * @returns {Promise<SimplifiedMNList>}
   */
  async getSimplifiedMNList() {
    if (this.needsUpdate()) {
      await this.updateMasternodeList();
    }

    return this.simplifiedMNList;
  }

  /**
   * Checks whether simplified masternode list needs update
   *
   * @private
   * @returns {boolean}
   */
  needsUpdate() {
    return Date.now() - this.options.updateInterval > this.lastUpdateDate;
  }

  /**
   * Updates simplified masternodes list. No need to call it manually
   *
   * @private
   */
  async updateMasternodeList() {
    const diff = await this.getSimplifiedMNListDiff();

    try {
      this.simplifiedMNList.applyDiff(diff);
    } catch (e) {
      if (e.message === 'Cannot apply diff: previous blockHash needs to equal the new diff\'s baseBlockHash') {
        this.reset();

        await this.updateMasternodeList();

        return;
      }

      throw e;
    }

    this.baseBlockHash = diff.blockHash;

    this.lastUpdateDate = Date.now();
  }

  /**
   * Fetches masternode diff from DAPI
   *
   * @private
   * @returns {Promise<SimplifiedMNListDiff>}
   */
  async getSimplifiedMNListDiff() {
    const blockHash = await this.jsonRpcTransport.request('getBestBlockHash');

    const rawSimplifiedMNListDiff = await this.jsonRpcTransport.request(
      'getMnListDiff',
      { baseBlockHash: this.baseBlockHash, blockHash },
      { addresses: [this.jsonRpcTransport.getLastUsedAddress()] },
    );

    return new SimplifiedMNListDiff(rawSimplifiedMNListDiff, this.options.network);
  }

  /**
   * Reset simplifiedMNList
   *
   * @private
   */
  reset() {
    this.simplifiedMNList = new SimplifiedMNList(undefined, this.options.network);

    this.lastUpdateDate = 0;

    this.baseBlockHash = SimplifiedMasternodeListProvider.NULL_HASH;
  }
}

SimplifiedMasternodeListProvider.NULL_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

module.exports = SimplifiedMasternodeListProvider;
