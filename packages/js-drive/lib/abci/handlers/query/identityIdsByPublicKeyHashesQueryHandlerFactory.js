const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');

const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const UnimplementedAbciError = require("../../errors/UnimplementedAbciError");

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} signedPublicKeyToIdentityIdRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {identityIdsByPublicKeyHashesQueryHandler}
 */
function identityIdsByPublicKeyHashesQueryHandlerFactory(
  signedPublicKeyToIdentityIdRepository,
  maxIdentitiesPerRequest,
  createQueryResponse,
  blockExecutionContextStack,
) {
  /**
   * @typedef identityIdsByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function identityIdsByPublicKeyHashesQueryHandler(params, { publicKeyHashes }, request) {
    if (publicKeyHashes && publicKeyHashes.length > maxIdentitiesPerRequest) {
      throw new InvalidArgumentAbciError(
        `Maximum number of ${maxIdentitiesPerRequest} requested items exceeded.`, {
          maxIdentitiesPerRequest,
        },
      );
    }

    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      const response = new GetIdentityIdsByPublicKeyHashesResponse();

      response.setIdentityIdsList(publicKeyHashes.map(() => cbor.encode([])));
      response.setMetadata(new ResponseMetadata());

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    if (request.prove) {
      throw new UnimplementedAbciError('Proofs are not implemented yet');
    }

    const response = createQueryResponse(GetIdentityIdsByPublicKeyHashesResponse, request.prove);

    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        signedPublicKeyToIdentityIdRepository.fetchBuffer(publicKeyHash)
      )),
    );

    const idsList = identityIds.map((ids) => {
      if (!ids) {
        return cbor.encode([]);
      }

      return ids;
    });

    response.setIdentityIdsList(idsList);

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityIdsByPublicKeyHashesQueryHandler;
}

module.exports = identityIdsByPublicKeyHashesQueryHandlerFactory;
