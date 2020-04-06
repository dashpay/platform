const jayson = require('jayson/promise');

const cbor = require('cbor');

const RPCError = require('../../rpcServer/RPCError');
const AbciResponseError = require('../../errors/AbciResponseError');

class DriveStateRepository {
  /**
   * @param options
   * @param {string} options.host
   * @param {number} options.port
   */
  constructor({ host, port }) {
    this.client = jayson.client.http({ host, port });
  }

  /**
   * Makes request to Drive and handle response
   *
   * @param {string} path
   * @param {Object} data
   *
   * @return {Promise<Buffer>}
   */
  async request(path, data = {}) {
    const encodedData = cbor.encode(data);

    const { result, error } = await this.client.request(
      'abci_query', {
        path,
        data: encodedData.toString('hex'),
      },
    );

    // Handle JSON RPC error
    if (error) {
      throw new RPCError(
        error.code || -32602, error.message || 'Internal error', error.data,
      );
    }

    // Check and handle ABCI errors
    const { response } = result;

    if (response.code === undefined || response.code === 0) {
      // no errors found return the serialized response value
      return Buffer.from(response.value, 'base64');
    }

    const { error: abciError } = JSON.parse(response.log);

    throw new AbciResponseError(response.code, abciError);
  }

  /**
   * Fetch serialized data contract
   *
   * @param {string} contractId
   *
   * @return {Promise<Buffer>}
   */
  async fetchDataContract(contractId) {
    return this.request(`/dataContracts/${contractId}`);
  }

  /**
   * Fetch serialized documents
   *
   * @param {string} contractId
   * @param {string} type - Documents type to fetch
   *
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   *
   * @return {Promise<Buffer[]>}
   */
  async fetchDocuments(contractId, type, options) {
    const serializedDocumentsArray = await this.request(
      `/dataContracts/${contractId}/documents/${type}`,
      options,
    );

    return cbor.decode(serializedDocumentsArray);
  }

  /**
   * Fetch serialized identity
   *
   * @param {string} id
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentity(id) {
    return this.request(`/identities/${id}`);
  }
}

module.exports = DriveStateRepository;
