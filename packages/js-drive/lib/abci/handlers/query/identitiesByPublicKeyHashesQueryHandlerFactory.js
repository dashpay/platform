const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {createQueryResponse} createQueryResponse
 * @return {identitiesByPublicKeyHashesQueryHandler}
 */
function identitiesByPublicKeyHashesQueryHandlerFactory(
  identityRepository,
  maxIdentitiesPerRequest,
  createQueryResponse,
) {
  /**
   * @typedef identitiesByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function identitiesByPublicKeyHashesQueryHandler(params, { publicKeyHashes }, request) {
    if (publicKeyHashes && publicKeyHashes.length > maxIdentitiesPerRequest) {
      throw new InvalidArgumentAbciError(
        `Maximum number of ${maxIdentitiesPerRequest} requested items exceeded.`, {
          maxIdentitiesPerRequest,
        },
      );
    }

    const response = createQueryResponse(GetIdentitiesByPublicKeyHashesResponse, request.prove);

    if (request.prove) {
      const proof = await identityRepository.proveManyByPublicKeyHashes(publicKeyHashes);

      response.getProof().setMerkleProof(proof.getValue());
    } else {
      const identitiesListResult = await identityRepository.fetchManyByPublicKeyHashes(
        publicKeyHashes,
      );

      response.setIdentitiesList(
        identitiesListResult.getValue().map((identity) => identity.toBuffer()),
      );
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identitiesByPublicKeyHashesQueryHandler;
}

module.exports = identitiesByPublicKeyHashesQueryHandlerFactory;
