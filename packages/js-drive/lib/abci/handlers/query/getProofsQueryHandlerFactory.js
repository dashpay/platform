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
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {IdentityStoreRepository} signedIdentityRepository
 * @param {DataContractStoreRepository} signedDataContractRepository
 * @param {DocumentRepository} signedDocumentRepository
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  blockExecutionContextStack,
  signedIdentityRepository,
  signedDataContractRepository,
  signedDocumentRepository,
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
    const blockExecutionContext = blockExecutionContextStack.getFirst();

    const signedBlockHeight = blockExecutionContext.getHeight();
    const signedCoreChainLockedHeight = blockExecutionContext.getCoreChainLockedHeight();

    const {
      quorumHash: signatureLlmqHash,
      stateSignature: signature,
    } = blockExecutionContext.getLastCommitInfo();

    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
      metadata: {
        height: signedBlockHeight.toNumber(),
        coreChainLockedHeight: signedCoreChainLockedHeight,
      },
    };

    if (documents && documents.length) {
      const documentsProof = await signedDocumentRepository
        .proveManyDocumentsFromDifferentContracts(documents);

      response.documentsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: documentsProof.getValue(),
      };
    }

    if (identityIds && identityIds.length) {
      const identitiesProof = await signedIdentityRepository.proveMany(
        identityIds.map((identityId) => Identifier.from(identityId)),
      );

      response.identitiesProof = {
        signatureLlmqHash,
        signature,
        merkleProof: identitiesProof.getValue(),
      };
    }

    if (dataContractIds && dataContractIds.length) {
      const dataContractsProof = await signedDataContractRepository.proveMany(
        dataContractIds.map((dataContractId) => Identifier.from(dataContractId)),
      );

      response.dataContractsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: dataContractsProof.getValue(),
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
