const jayson = require('jayson/promise');
const AbstractDashDriveAdapter = require('./AbstractDashDriveAdapter');
const RPCError = require('../../rpcServer/RPCError');

class DashDriveAdapter extends AbstractDashDriveAdapter {
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
   * Makes request to DashDrive and handles response
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
   * Add State Transition Packet to DashDrive storage
   * @param {string} packet - raw data packet serialized to hex string
   * @return {Promise<string>} - packet id
   */
  addSTPacket(packet) {
    return this.request('addSTPacket', { packet });
  }

  /**
   * Fetch DAP Contract from DashDrive State View
   * @param {string} dapId
   * @return {Promise<Object>} - Dap contract
   */
  fetchDapContract(dapId) {
    return this.request('fetchDapContract', { dapId });
  }

  /**
   * Fetch DAP Objects from DashDrive State View
   * @param {string} dapId
   * @param {string} type - Dap objects type to fetch
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   * @return {Promise<Object[]>}
   */
  fetchDapObjects(dapId, type, options) {
    return this.request('fetchDapObjects', { dapId, type, options });
  }
}

module.exports = DashDriveAdapter;
