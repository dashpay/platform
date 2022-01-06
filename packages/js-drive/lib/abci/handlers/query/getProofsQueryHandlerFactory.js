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
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {blockExecutionContextStack} blockExecutionContextStack,
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  blockExecutionContext,
  blockExecutionContextStack,
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
    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
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
      stateSignature: signature,
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
        ...previousRootTree.getFullProofForOneLeaf(previousDocumentsStoreRootTreeLeaf, documentIds),
      };
    }

    if (identityIds && identityIds.length) {
      response.identitiesProof = {
        signatureLlmqHash,
        signature,
        ...previousRootTree.getFullProofForOneLeaf(
          previousIdentitiesStoreRootTreeLeaf, identityIds,
        ),
      };
    }

    if (dataContractIds && dataContractIds.length) {
      response.dataContractsProof = {
        signatureLlmqHash,
        signature,
        ...previousRootTree.getFullProofForOneLeaf(
          previousDataContractsStoreRootTreeLeaf, dataContractIds,
        ),
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
