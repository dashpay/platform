const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesContractKeysRequest,
  },
} = require('@dashevo/dapi-grpc');
const { IdentityPublicKey } = require('@dashevo/wasm-dpp');

const GetIdentitiesContractKeysResponse = require('./GetIdentitiesContractKeysResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentitiesContractKeys}
 */
function getIdentitiesContractKeysFactory(grpcTransport) {
  /**
   * Fetch the identities by public key hashes
   * @typedef {getIdentitiesContractKeys}
   * @param {Buffer[]} identitiesIds
   * @param {Buffer} contractId
   * @param {IdentityPublicKey.PURPOSES[]} keyPurposes
   * @param {string | null} documentTypeName
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentitiesContractKeysResponse>}
   */
  async function getIdentitiesContractKeys(
    identitiesIds,
    contractId,
    keyPurposes,
    documentTypeName = null,
    options = {},
  ) {
    const { GetIdentitiesContractKeysRequestV0 } = GetIdentitiesContractKeysRequest;
    const getIdentitiesContractKeysRequest = new GetIdentitiesContractKeysRequest();

    // eslint-disable-next-line no-param-reassign
    identitiesIds = identitiesIds.map((id) => {
      if (Buffer.isBuffer(id)) {
        // eslint-disable-next-line no-param-reassign
        id = Buffer.from(id);
      }

      return id;
    });

    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getIdentitiesContractKeysRequest.setV0(
      new GetIdentitiesContractKeysRequestV0()
        .setProve(!!options.prove)
        .setIdentitiesIdsList(identitiesIds)
        .setContractId(contractId)
        .setPurposesList(keyPurposes)
        .setDocumentTypeName(documentTypeName),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentitiesContractKeysResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentitiesContractKeys',
          getIdentitiesContractKeysRequest,
          options,
        );

        return GetIdentitiesContractKeysResponse
          .createFromProto(getIdentitiesContractKeysResponse);
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

  return getIdentitiesContractKeys;
}

module.exports = getIdentitiesContractKeysFactory;
