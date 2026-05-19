import dapiGrpc from '@dashevo/dapi-grpc';
import wrappersPb from 'google-protobuf/google/protobuf/wrappers_pb.js';
import GetEpochsInfoResponse from './GetEpochsInfoResponse.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';

const {
  v0: {
    PlatformPromiseClient,
    GetEpochsInfoRequest,
  },
} = dapiGrpc;

const { UInt32Value } = wrappersPb;

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getEpochsInfo}
 */
function getEpochsInfoFactory(grpcTransport) {
  /**
   * Fetch the epoch info
   * @typedef {getEpochsInfo}
   * @param {number} startEpoch
   * @param {number} count
   * @param {DAPIClientOptions & {prove: boolean, ascending: boolean}} [options]
   * @returns {Promise<GetEpochsInfoResponse>}
   */
  async function getEpochsInfo(startEpoch, count, options = {}) {
    const { GetEpochsInfoRequestV0 } = GetEpochsInfoRequest;
    const getEpochInfosRequest = new GetEpochsInfoRequest();
    getEpochInfosRequest.setV0(
      new GetEpochsInfoRequestV0()
        .setStartEpoch(typeof startEpoch === 'number' ? new UInt32Value([startEpoch]) : undefined)
        .setCount(count)
        .setAscending(!!options.ascending)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getEpochsInfoResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getEpochsInfo',
          getEpochInfosRequest,
          options,
        );

        return GetEpochsInfoResponse.createFromProto(getEpochsInfoResponse);
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

  return getEpochsInfo;
}

export default getEpochsInfoFactory;
