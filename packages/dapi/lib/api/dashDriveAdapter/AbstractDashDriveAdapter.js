/* eslint no-unused-vars: off, class-methods-use-this: off */
class AbstractDashDriveAdapter {
  /**
   * Add State Transition Packet to DashDrive storage
   * @param {string} rawStateTransition - serialized state transition header
   * @param {string} rawSTPacket - raw data packet serialized to hex string
   * @return {Promise<string>} - packet id
   */
  addSTPacket(rawStateTransition, rawSTPacket) {
    throw new Error('Not implemented');
  }

  /**
   * Fetch DAP Contract from DashDrive State View
   * @param {string} contractId
   * @return {Promise<Object>} - Dap contract
   */
  fetchDapContract(contractId) {
    throw new Error('Not implemented');
  }

  /**
   * Fetch DAP Objects from DashDrive State View
   * @param {string} contractId
   * @param {string} type - Dap objects type to fetch
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   * @return {Promise<Object[]>}
   */
  fetchDapObjects(contractId, type, options) {
    throw new Error('Not implemented');
  }
}

module.exports = AbstractDashDriveAdapter;
