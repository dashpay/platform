const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');

/**
 *
 * @param {RootTree} previousRootTree
 * @param {DocumentsStoreRootTreeLeaf} previousDocumentsStoreRootTreeLeaf
 * @param {IdentitiesStoreRootTreeLeaf} previousIdentitiesStoreRootTreeLeaf
 * @param {DataContractsStoreRootTreeLeaf} previousDataContractsStoreRootTreeLeaf
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext,
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  previousRootTree,
  previousDocumentsStoreRootTreeLeaf,
  previousIdentitiesStoreRootTreeLeaf,
  previousDataContractsStoreRootTreeLeaf,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef getProofsQueryHandler
   * @param params
   * @param callArguments
   * @param {Identifier[]} callArguments.identityIds
   * @param {Identifier[]} callArguments.documentIds
   * @param {Identifier[]} callArguments.dataContractIds
   * @return {Promise<ResponseQuery>}
   */
  async function getProofsQueryHandler(params, {
    identityIds,
    documentIds,
    dataContractIds,
  }) {
    if (blockExecutionContext.isEmpty() || previousBlockExecutionContext.isEmpty()) {
      return new ResponseQuery({
        value: await cbor.encodeAsync({
          documentsProof: null,
          identitiesProof: null,
          dataContractsProof: null,
          metadata: {
            height: 0,
            coreChainLockedHeight: 0,
          },
        }),
      });
    }

    const {
      height: previousBlockHeight,
      coreChainLockedHeight: previousCoreChainLockedHeight,
    } = previousBlockExecutionContext.getHeader();

    const {
      quorumHash: signatureLlmqHash,
      signature,
    } = blockExecutionContext.getLastCommitInfo();

    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
      metadata: {
        height: previousBlockHeight.toNumber(),
        coreChainLockedHeight: previousCoreChainLockedHeight,
      },
    };

    if (documentIds && documentIds.length) {
      response.documentsProof = {
        signatureLlmqHash,
        signature,
        ...previousRootTree.getFullProof(previousDocumentsStoreRootTreeLeaf, documentIds),
      };
    }

    if (identityIds && identityIds.length) {
      response.identitiesProof = {
        signatureLlmqHash,
        signature,
        ...previousRootTree.getFullProof(previousIdentitiesStoreRootTreeLeaf, identityIds),
      };
    }

    if (dataContractIds && dataContractIds.length) {
      response.dataContractsProof = {
        signatureLlmqHash,
        signature,
        ...previousRootTree.getFullProof(previousDataContractsStoreRootTreeLeaf, dataContractIds),
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
