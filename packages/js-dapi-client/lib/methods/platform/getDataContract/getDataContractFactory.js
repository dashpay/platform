const {
  v0: {
    PlatformPromiseClient,
    GetDataContractRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetDataContractResponse = require('./GetDataContractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getDataContract}
 */
function getDataContractFactory(grpcTransport) {
  /**
   * Fetch Data Contract by id
   *
   * @typedef {getDataContract}
   * @param {Buffer} contractId
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetDataContractResponse>}
   */
  async function getDataContract(contractId, options = {}) {
    const getDataContractRequest = new GetDataContractRequest();

    // need to convert objects inherited from Buffer to pure buffer as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getDataContractRequest.setId(contractId);
    getDataContractRequest.setProve(!!options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getDataContractResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getDataContract',
          getDataContractRequest,
          options,
        );

        return GetDataContractResponse.createFromProto(getDataContractResponse);
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

  return getDataContract;
}

module.exports = getDataContractFactory;
