const {
  v0: {
    PlatformPromiseClient,
    GetIdentityRequest,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const GetIdentityResponse = require('./GetIdentityResponse');
const Metadata = require('../response/Metadata');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentity}
 */
function getIdentityFactory(grpcTransport) {
  /**
   * Fetch the identity by id
   *
   * @typedef {getIdentity}
   * @param {Buffer} id
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<GetIdentityResponse>}
   */
  async function getIdentity(id, options = {}) {
    const getIdentityRequest = new GetIdentityRequest();
    // need to convert objects inherited from Buffer to pure buffer as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (Buffer.isBuffer(id)) {
      // eslint-disable-next-line no-param-reassign
      id = Buffer.from(id);
    }

    getIdentityRequest.setId(id);

    let getIdentityResponse;
    try {
      getIdentityResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getIdentity',
        getIdentityRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return new GetIdentityResponse(null, new Metadata({ height: 0, coreChainLockedHeight: 0 }));
      }

      throw e;
    }

    return GetIdentityResponse.createFromProto(getIdentityResponse);
  }

  return getIdentity;
}

module.exports = getIdentityFactory;
