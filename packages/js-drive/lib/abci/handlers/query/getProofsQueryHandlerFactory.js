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
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
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

    const blockExecutionContext = blockExecutionContextStack.getFirst();
    const signedBlockExecutionContext = blockExecutionContextStack.getLast();

    const {
      height: signedBlockHeight,
      coreChainLockedHeight: signedCoreChainLockedHeight,
    } = signedBlockExecutionContext.getHeader();

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

    if (documentIds && documentIds.length) {
      response.documentsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: Buffer.from([1]),
      };
    }

    if (identityIds && identityIds.length) {
      response.identitiesProof = {
        signatureLlmqHash,
        signature,
        merkleProof: Buffer.from([1]),
      };
    }

    if (dataContractIds && dataContractIds.length) {
      response.dataContractsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: Buffer.from([1]),
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
