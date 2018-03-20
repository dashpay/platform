/**
 * This module responsibility is to obtain masternode IPs in order to
 * provide those IPs for DAPIService, which provides an interface for making
 * requests to DAPI.
 *  @module MNDiscoveryService
 */

const sample = require('lodash/sample');
const MasternodeListProvider = require('./MasternodeListProvider');

const MNDiscoveryService = {
  /**
   * @returns {Promise<Masternode>}
   */
  async getRandomMasternode() {
    const MNList = await this.masternodeListProvider.getMNList();
    return sample(MNList);
  },
  /**
   * @returns {Promise<Array<Masternode>>}
   */
  getMNList() {
    return this.masternodeListProvider.getMNList();
  },
  /**
   * @private
   * @protected
   * For test purposes only: tests wraps .getMNList() method of that object to ensure
   * it was called.
   */
  masternodeListProvider: new MasternodeListProvider(),
  /**
   * @private
   * Deletes cached MNList and resets it back to initial seed.
   * Used in MNDiscoveryService tests; No need to call that method manually.
   * @return void
   */
  reset() {
    this.masternodeListProvider = new MasternodeListProvider();
  },
};

module.exports = MNDiscoveryService;
