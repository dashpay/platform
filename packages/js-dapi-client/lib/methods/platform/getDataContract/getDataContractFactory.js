const {
  v0: {
    PlatformPromiseClient,
    GetDataContractRequest,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const bs58 = require('bs58');

const GetDataContractResponse = require('./GetDataContractResponse');
const NotFoundError = require('../../errors/NotFoundError');

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
   * @param {DAPIClientOptions} [options]
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

    let getDataContractResponse;
    try {
      getDataContractResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getDataContract',
        getDataContractRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        throw new NotFoundError(`DataContract ${bs58.encode(contractId)} is not found`);
      }

      throw e;
    }

    return GetDataContractResponse.createFromProto(getDataContractResponse);
  }

  return getDataContract;
}

module.exports = getDataContractFactory;
