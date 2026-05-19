import dapiGrpc from '@dashevo/dapi-grpc';
import GetDataContractResponse from './GetDataContractResponse.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';

const {
  v0: {
    PlatformPromiseClient,
    GetDataContractRequest,
  },
} = dapiGrpc;

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getDataContract}
 */
function getDataContractFactory(grpcTransport) {
  /**
   * Fetch Data Contract by id
   * @typedef {getDataContract}
   * @param {Uint8Array} contractId
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetDataContractResponse>}
   */
  async function getDataContract(contractId, options = {}) {
    const { GetDataContractRequestV0 } = GetDataContractRequest;
    const getDataContractRequest = new GetDataContractRequest();

    // need to convert objects inherited from Uint8Array to pure Uint8Array as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (contractId instanceof Uint8Array) {
      // eslint-disable-next-line no-param-reassign
      contractId = new Uint8Array(contractId);
    }

    getDataContractRequest.setV0(
      new GetDataContractRequestV0()
        .setId(contractId)
        .setProve(!!options.prove),
    );

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

export default getDataContractFactory;
