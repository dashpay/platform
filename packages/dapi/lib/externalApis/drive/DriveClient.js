const jayson = require('jayson/promise');

const RPCError = require('../../rpcServer/RPCError');
const createGrpcErrorFromDriveResponse = require('../../grpcServer/handlers/createGrpcErrorFromDriveResponse');

class DriveClient {
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
   * @param {Uint8Array} data
   *
   * @return {Promise<Buffer>}
   */
  async request(path, data) {
    const requestOptions = {
      path,
      data: Buffer.from(data).toString('hex'),
    };

    const { result, error } = await this.client.request(
      'abci_query',
      requestOptions,
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

    throw await createGrpcErrorFromDriveResponse(response.code, response.info);
  }

  /**
   * Fetch serialized data contract
   *
   * @param {GetDataContractRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchDataContract(request) {
    return this.request(
      '/dataContract',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized data contracts
   *
   * @param {GetDataContractsRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchDataContracts(request) {
    return this.request(
      '/dataContracts',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized data contract
   *
   * @param {GetDataContractHistoryRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchDataContractHistory(request) {
    return this.request(
      '/dataContractHistory',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized documents
   *
   * @param {GetDocumentsRequest} request
   *
   * @return {Promise<Buffer[]>}
   */
  async fetchDocuments(request) {
    return this.request(
      '/dataContract/documents',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identity
   *
   * @param {GetIdentityRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentity(request) {
    return this.request(
      '/identity',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identities
   *
   * @param {GetIdentitiesRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentities(request) {
    return this.request(
      '/identities',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identity balance
   *
   * @param {GetIdentityBalanceRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentityBalance(request) {
    return this.request(
      '/identity/balance',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identity balance and revision
   *
   * @param {GetIdentityBalanceAndRevisionRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentityBalanceAndRevision(request) {
    return this.request(
      '/identity/balanceAndRevision',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identity keys
   *
   * @param {GetIdentityKeysRequest} request
   *
   * @return {Promise<Buffer>}
   */
  async fetchIdentityKeys(request) {
    return this.request(
      '/identity/keys',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identity by its public key hashes
   *
   * @param {GetIdentityByPublicKeyHashesRequest} request
   *
   * @return {Promise<Buffer[]>}
   */
  async fetchIdentityByPublicKeyHashes(request) {
    return this.request(
      '/identity/by-public-key-hash',
      request.serializeBinary(),
    );
  }

  /**
   * Fetch serialized identities by its public key hashes
   *
   * @param {GetIdentitiesByPublicKeyHashesRequest} request
   *
   * @return {Promise<Buffer[]>}
   */
  async fetchIdentitiesByPublicKeyHashes(request) {
    return this.request(
      '/identities/by-public-key-hash',
      request.serializeBinary(),
    );
  }

  /**
   *  Fetch proofs by ids
   *
   * @param {GetProofsRequest} request

   * @return {Promise<{data: Buffer}>}
   */
  async fetchProofs(request) {
    return this.request(
      '/proofs',
      request.serializeBinary(),
    );
  }
}

module.exports = DriveClient;
