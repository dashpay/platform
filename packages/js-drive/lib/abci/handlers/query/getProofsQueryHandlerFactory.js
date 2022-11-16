const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

/**
 *
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {DocumentRepository} documentRepository
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  latestBlockExecutionContext,
  identityRepository,
  dataContractRepository,
  documentRepository,
) {
  /**
   * @typedef getProofsQueryHandler
   * @param params
   * @param callArguments
   * @param {Buffer[]} callArguments.identityIds
   * @param {Buffer[]} callArguments.dataContractIds
   * @param {{dataContractId: Buffer, documentId: Buffer, type: string}[]} documents
   * @return {Promise<ResponseQuery>}
   */
  async function getProofsQueryHandler(params, {
    identityIds,
    dataContractIds,
    documents,
  }) {
    const blockHeight = latestBlockExecutionContext.getHeight();
    const coreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();
    const time = latestBlockExecutionContext.getTime();
    const version = latestBlockExecutionContext.getVersion();
    const {
      quorumHash: signatureLlmqHash,
      stateSignature: signature,
    } = latestBlockExecutionContext.getLastCommitInfo();
    const round = latestBlockExecutionContext.getRound();

    const timeObject = time.toJSON();
    timeObject.seconds = Number(timeObject.seconds);

    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
      metadata: {
        height: blockHeight.toNumber(),
        coreChainLockedHeight,
        blockTime: timeObject,
        protocolVersion: version.app.toNumber(),
      },
    };

    if (documents && documents.length) {
      const documentsProof = await documentRepository
        .proveManyDocumentsFromDifferentContracts(documents);

      response.documentsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: documentsProof.getValue(),
        round,
      };
    }

    if (identityIds && identityIds.length) {
      const identitiesProof = await identityRepository.proveMany(
        identityIds.map((identityId) => Identifier.from(identityId)),
      );

      response.identitiesProof = {
        signatureLlmqHash,
        signature,
        merkleProof: identitiesProof.getValue(),
        round,
      };
    }

    if (dataContractIds && dataContractIds.length) {
      const dataContractsProof = await dataContractRepository.proveMany(
        dataContractIds.map((dataContractId) => Identifier.from(dataContractId)),
      );

      response.dataContractsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: dataContractsProof.getValue(),
        round,
      };
    }

    return new ResponseQuery({
      value: await cbor.encodeAsync(
        response,
      ),
    });
  }

  return getProofsQueryHandler;
}

module.exports = getProofsQueryHandlerFactory;
