/**
 * This module provides list of masternode addresses.
 * No need to use this module manually - it's part of MNDiscoveryService.
 * It is written as class for testability purposes - there is need to be a way to
 * reset internal state of object.
 * @module MasternodeListProvider
 */

const RPCClient = require('../../utils/RPCClient');
const sample = require('lodash/sample');
const config = require('../../config/index');

/**
 @typedef {object} Masternode
 @property {string} vin
 @property {string} status
 @property {number} rank
 @property {string} ip - including port
 @property {string} protocol - protocol version
 @property {string} payee
 @property {number} activeseconds
 @property {number} lastseen
 */

class MasternodeListProvider {
  constructor() {
    /**
     * Masternode list. Initial masternode list is DNS seed from SDK config.
     * @type Array<Masternode>
     */
    this.masternodeList = config.DAPIDNSSeeds.slice();
    this.lastUpdateDate = 0;
  }
  /**
   * @private
   * Fetches masternode list from DAPI.
   * @returns {Promise<Array<Masternode>>}
   */
  async fetchMNList() {
    const randomMasternode = sample(this.masternodeList);
    const MNList = await RPCClient.request({
      host: randomMasternode.ip,
      port: config.Api.port,
    }, 'getMNList', []);
    if (!MNList) {
      throw new Error('Failed to fetch masternodes list');
    }
    return MNList;
  }
  /**
   * @private
   * Updates masternodes list. No need to call it manually
   * @returns {Promise<void>}
   */
  async updateMNList() {
    const newMNList = await this.fetchMNList();
    // If mn list was updated
    if (newMNList.length) {
      this.masternodeList = newMNList;
    }
    this.lastUpdateDate = Date.now();
  }
  /**
   * @private
   * Checks whether masternode list needs update
   * @returns {boolean}
   */
  needsUpdate() {
    return Date.now() - config.MNListUpdateInterval > this.lastUpdateDate;
  }

  /**
   * Returns masternode list
   * @returns {Promise<Array<Masternode>>}
   */
  async getMNList() {
    if (this.needsUpdate()) {
      await this.updateMNList();
    }
    return this.masternodeList;
  }
}

module.exports = MasternodeListProvider;
