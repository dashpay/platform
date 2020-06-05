const {
  PlatformPromiseClient,
  GetIdentityIdByFirstPublicKeyRequest,
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityIdByFirstPublicKey}
 */
function getIdentityIdByFirstPublicKeyFactory(grpcTransport) {
  /**
   * Fetch the identity id by public key hash
   *
   * @typedef {getIdentityIdByFirstPublicKey}
   * @param {string} publicKeyHash
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<!string|null>}
   */
  async function getIdentityIdByFirstPublicKey(publicKeyHash, options = {}) {
    const getIdentityIdByFirstPublicKeyRequest = new GetIdentityIdByFirstPublicKeyRequest();
    getIdentityIdByFirstPublicKeyRequest.setPublicKeyHash(Buffer.from(publicKeyHash, 'hex'));

    let getIdentityIdByFirstPublicKeyResponse;
    try {
      getIdentityIdByFirstPublicKeyResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getIdentityIdByFirstPublicKey',
        getIdentityIdByFirstPublicKeyRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    return getIdentityIdByFirstPublicKeyResponse.getId();
  }

  return getIdentityIdByFirstPublicKey;
}

module.exports = getIdentityIdByFirstPublicKeyFactory;
