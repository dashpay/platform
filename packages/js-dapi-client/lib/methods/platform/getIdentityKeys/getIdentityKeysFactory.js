const {
  v0: {
    PlatformPromiseClient,
    GetIdentityKeysRequest,
    KeyRequestType,
    SpecificKeys,
  },
} = require('@dashevo/dapi-grpc');
const { UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

const GetIdentityKeysResponse = require('./GetIdentityKeysResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityKeys}
 */
function getIdentityKeysFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getIdentityKeys}
   * @param {Buffer} identityId
   * @param {number[]} keyIds
   * @param {number} limit
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityKeysResponse>}
   */
  async function getIdentityKeys(identityId, keyIds, limit = 100, options = {}) {
    const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;
    const getIdentityKeysRequest = new GetIdentityKeysRequest();

    if (Buffer.isBuffer(identityId)) {
      // eslint-disable-next-line no-param-reassign
      identityId = Buffer.from(identityId);
    }

    getIdentityKeysRequest.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(new KeyRequestType()
          .setSpecificKeys(new SpecificKeys().setKeyIdsList(keyIds)))
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

module.exports = getIdentityKeysFactory;
