const jayson = require('jayson/promise');
const AbstractDriveAdapter = require('./AbstractDriveAdapter');
const RPCError = require('../../rpcServer/RPCError');

class DriveAdapter extends AbstractDriveAdapter {
  /**
   * @param options
   * @param {string} options.host
   * @param {number} options.port
   */
  constructor(options) {
    super();
    const { host, port } = options;
    this.client = jayson.client.http({ host, port });
  }

  /**
   * Makes request to Drive and handles response
   * @param {string} method
   * @param {Object} params
   * @return {Promise<*>}
   */
  async request(method, params) {
    const { result, error } = await this.client.request(method, params);
    if (error) {
      throw new RPCError(error.code || -32602, error.message || 'Internal error');
    }
    return result;
  }

  /**
   * Add State Transition Packet to Drive storage
   * @param {string} rawStateTransition - special transaction
   * @param {string} rawSTPacket - raw data packet serialized to hex string
   * @return {Promise<undefined>}
   */
  addSTPacket(rawStateTransition, rawSTPacket) {
    return this.request('addSTPacket', {
      stateTransition: rawStateTransition,
      stPacket: rawSTPacket,
    });
  }

  /**
   * Fetch DAP Contract from Drive State View
   * @param {string} contractId
   * @return {Promise<Object>} - Contract
   */
  fetchContract(contractId) {
    return this.request('fetchContract', { contractId });
  }

  /**
   * Fetch DAP Objects from Drive State View
   * @param {string} contractId
   * @param {string} type - Documents type to fetch
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   * @return {Promise<Object[]>}
   */
  fetchDocuments(contractId, type, options) {
    return this.request('fetchDocuments', { contractId, type, options });
  }
}

module.exports = DriveAdapter;
