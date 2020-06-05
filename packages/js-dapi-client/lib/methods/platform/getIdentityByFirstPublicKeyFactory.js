const {
  PlatformPromiseClient,
  GetIdentityByFirstPublicKeyRequest,
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityByFirstPublicKey}
 */
function getIdentityByFirstPublicKeyFactory(grpcTransport) {
  /**
   * Fetch the identity by public key hash
   *
   * @typedef {getIdentityByFirstPublicKey}
   * @param {string} publicKeyHash
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<!Buffer|null>}
   */
  async function getIdentityByFirstPublicKey(publicKeyHash, options = {}) {
    const getIdentityByFirstPublicKeyRequest = new GetIdentityByFirstPublicKeyRequest();
    getIdentityByFirstPublicKeyRequest.setPublicKeyHash(Buffer.from(publicKeyHash, 'hex'));

    let getIdentityByFirstPublicKeyResponse;
    try {
      getIdentityByFirstPublicKeyResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getIdentityByFirstPublicKey',
        getIdentityByFirstPublicKeyRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const serializedIdentityBinaryArray = getIdentityByFirstPublicKeyResponse.getIdentity();
    let identity = null;

    if (serializedIdentityBinaryArray) {
      identity = Buffer.from(serializedIdentityBinaryArray);
    }

    return identity;
  }

  return getIdentityByFirstPublicKey;
}

module.exports = getIdentityByFirstPublicKeyFactory;
