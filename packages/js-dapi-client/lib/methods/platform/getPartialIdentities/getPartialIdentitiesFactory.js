const {
  v0: {
    PlatformPromiseClient,
    GetPartialIdentitiesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetPartialIdentitiesResponse = require('./GetPartialIdentitiesResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getPartialIdentities}
 */
function getPartialIdentitiesFactory(grpcTransport) {
  /**
   * Fetch the identities by public key hashes
   * @typedef {getPartialIdentities}
   * @param {Buffer[]} ids
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetPartialIdentitiesResponse>}
   */
  async function getPartialIdentities(ids, options = {}) {
    const { GetPartialIdentitiesRequestV0 } = GetPartialIdentitiesRequest;
    const getPartialIdentitiesRequest = new GetPartialIdentitiesRequest();

    // eslint-disable-next-line no-param-reassign
    ids = ids.map((id) => {
      if (Buffer.isBuffer(id)) {
        // eslint-disable-next-line no-param-reassign
        id = Buffer.from(id);
      }

      return id;
    });

    getPartialIdentitiesRequest.setV0(
      new GetPartialIdentitiesRequestV0()
        .setProve(!!options.prove)
        .setIdsList(ids),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getPartialIdentitiesResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getPartialIdentities',
          getPartialIdentitiesRequest,
          options,
        );

        return GetPartialIdentitiesResponse
          .createFromProto(getPartialIdentitiesResponse);
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

  return getPartialIdentities;
}

module.exports = getPartialIdentitiesFactory;
