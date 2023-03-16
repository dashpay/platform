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

const NotFoundAbciError = require('../../errors/NotFoundAbciError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {createQueryResponse} createQueryResponse
 * @param {WebAssembly.Instance} dppWasm
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  identityRepository,
  createQueryResponse,
  dppWasm,
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
      identifier = new dppWasm.Identifier(id);
    } catch (e) {
      if (e instanceof dppWasm.IdentifierError) {
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
