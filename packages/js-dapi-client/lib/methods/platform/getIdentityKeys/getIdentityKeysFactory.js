import dapiGrpc from '@dashevo/dapi-grpc';
import wrappersPb from 'google-protobuf/google/protobuf/wrappers_pb.js';
import GetIdentityKeysResponse from './GetIdentityKeysResponse.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';

const {
  v0: {
    PlatformPromiseClient,
    GetIdentityKeysRequest,
    KeyRequestType,
    SpecificKeys,
    AllKeys,
  },
} = dapiGrpc;

const { UInt32Value } = wrappersPb;

const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityKeys}
 */
function getIdentityKeysFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getIdentityKeys}
   * @param {Uint8Array} identityId
   * @param {number[]=} keyIds
   * @param {number} limit
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityKeysResponse>}
   */
  async function getIdentityKeys(identityId, keyIds, limit = 100, options = {}) {
    if (identityId instanceof Uint8Array) {
      // eslint-disable-next-line no-param-reassign
      identityId = new Uint8Array(identityId);
    }

    const getIdentityKeysRequest = new GetIdentityKeysRequest();
    const requestType = new KeyRequestType();

    if (keyIds) {
      requestType.setSpecificKeys(new SpecificKeys().setKeyIdsList(keyIds));
    } else {
      requestType.setAllKeys(new AllKeys());
    }

    getIdentityKeysRequest.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(requestType)
        .setLimit(new UInt32Value([limit]))
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityKeysResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityKeys',
          getIdentityKeysRequest,
          options,
        );

        return GetIdentityKeysResponse
          .createFromProto(getIdentityKeysResponse);
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

  return getIdentityKeys;
}

export default getIdentityKeysFactory;
