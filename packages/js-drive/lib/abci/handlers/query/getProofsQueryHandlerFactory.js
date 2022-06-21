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
 * @param {fetchDataContract} fetchSignedDataContract
 * @param {proveDocuments} proveSignedDocuments
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  blockExecutionContextStack,
  signedIdentityRepository,
  signedDataContractRepository,
  // eslint-disable-next-line no-unused-vars
  fetchSignedDataContract,
  // eslint-disable-next-line no-unused-vars
  proveSignedDocuments,
) {
  /**
   * @typedef getProofsQueryHandler
   * @param params
   * @param callArguments
   * @param {Buffer[]} callArguments.identityIds
   * @param {Buffer[]} callArguments.dataContractIds
   * @param {{dataContractId: Buffer, documentId: Identifier, type: string}[]} documents
   * @return {Promise<ResponseQuery>}
   */
  async function getProofsQueryHandler(params, {
    identityIds,
    dataContractIds,
    documents,
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

    if (documents && documents.length) {
      response.documentsProof = {
        signatureLlmqHash,
        signature,
        merkleProof: Buffer.from([1]),
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
