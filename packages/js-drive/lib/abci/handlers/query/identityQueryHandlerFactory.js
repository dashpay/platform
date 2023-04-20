const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityResponse,
  },
} = require('@dashevo/dapi-grpc');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {createQueryResponse} createQueryResponse
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  identityRepository,
  createQueryResponse,
) {
  /**
   * @typedef identityQueryHandler
   * @param {Object} params
   * @param {Object} options
   * @param {Buffer} options.id
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler(params, { id }, request) {
    let identifier;
    try {
      identifier = new Identifier(id);
    } catch (e) {
      if (e instanceof IdentifierError) {
        throw new InvalidArgumentAbciError('id must be a valid identifier (32 bytes long)');
      }

      throw e;
    }

    const response = createQueryResponse(GetIdentityResponse, request.prove);

    if (request.prove) {
      const proof = await identityRepository.prove(identifier);

      response.getProof().setMerkleProof(proof.getValue());
    } else {
      const identityResult = await identityRepository.fetch(identifier);

      if (identityResult.isNull()) {
        throw new NotFoundAbciError('Identity not found');
      }

      response.setIdentity(identityResult.getValue().toBuffer());
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
