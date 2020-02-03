/**
 * This module's responsibility is to obtain masternode IPs in order to
 * provide those IPs for DAPIService, which provides an interface for making
 * requests to DAPI.
 *  @module MNDiscoveryService
 */

const sample = require('lodash/sample');
const MasternodeListProvider = require('./MasternodeListProvider');

class MNDiscovery {
  /**
   * @class
   * @param {Array} [seeds] - optional. Seeds to use. If nothing passed, default seeds will be used.
   * Default will be fine in most of situations.
   * @param {number} [port] - optional. Default port for connection to the DAPI
   */
  constructor(seeds, port) {
    /**
     * @private
     * @protected
     * For test purposes only: tests wraps .getMNList() method of that object to ensure
     * it was called.
     */
    this.masternodeListProvider = new MasternodeListProvider(seeds, port);
    /**
     * @private
     * @protected
     * @type {Array}
     */
    this.seeds = seeds;
  }

  /**
   * @returns {Promise<SimplifiedMNListEntry>}
   */
  async getRandomMasternode(excludedIps) {
    let MNList = await this.masternodeListProvider.getMNList();
    if (Array.isArray(excludedIps)) {
      MNList = MNList.filter((mn) => excludedIps.indexOf(mn.service.split(':')[0]) < 0);
    }
    return sample(MNList);
  }

  /**
   * @returns {Promise<Array<SimplifiedMNListEntry>>}
   */
  getMNList() {
    return this.masternodeListProvider.getMNList();
  }

  /**
   * @private
   * Deletes cached MNList and resets it back to initial seed.
   * Used in MNDiscovery tests; No need to call that method manually.
   * @return void
   */
  reset() {
    this.masternodeListProvider = new MasternodeListProvider(this.seeds);
  }
}

module.exports = MNDiscovery;
