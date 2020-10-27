const {
  v0: {
    PlatformPromiseClient,
    GetIdentityRequest,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

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
   * @returns {Promise<!Buffer|null>}
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
        return null;
      }

      throw e;
    }

    const serializedIdentityBinaryArray = getIdentityResponse.getIdentity();
    let identity = null;

    if (serializedIdentityBinaryArray) {
      identity = Buffer.from(serializedIdentityBinaryArray);
    }

    return identity;
  }

  return getIdentity;
}

module.exports = getIdentityFactory;
