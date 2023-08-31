const {
  v0: {
    PlatformPromiseClient,
    GetDataContractHistoryRequest,
  },
} = require('@dashevo/dapi-grpc');

const { UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

const GetDataContractHistoryResponse = require('./GetDataContractHistoryResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getDataContractHistory}
 */
function getDataContractHistoryFactory(grpcTransport) {
  /**
   * Fetch Data Contract by id
   *
   * @typedef {getDataContractHistory}
   * @param {Buffer} contractId
   * @param {number} [startAtMs]
   * @param {number} [limit]
   * @param {number} [offset]
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetDataContractHistoryResponse>}
   */
  async function getDataContractHistory(
    contractId,
    startAtMs = 0,
    limit = 10,
    offset = 0,
    options = {},
  ) {
    const getDataContractHistoryRequest = new GetDataContractHistoryRequest();

    // need to convert objects inherited from Buffer to pure buffer as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getDataContractHistoryRequest.setId(contractId);
    getDataContractHistoryRequest.setStartAtMs(startAtMs);
    getDataContractHistoryRequest.setLimit(new UInt32Value([limit]));
    getDataContractHistoryRequest.setOffset(new UInt32Value([offset]));
    getDataContractHistoryRequest.setProve(!!options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getDataContractHistoryResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getDataContractHistory',
          getDataContractHistoryRequest,
          options,
        );

        return GetDataContractHistoryResponse.createFromProto(getDataContractHistoryResponse);
      } catch (e) {
        if (e instanceof InvalidResponseError) {
          lastError = e;
        } else {
          throw e;
        }
      }
    }

    // If we made it past the cycle it means that the retry didn't work,
    // and we're throwing the last error encountered
    throw lastError;
  }

  return getDataContractHistory;
}

module.exports = getDataContractHistoryFactory;
