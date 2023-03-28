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
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {DocumentRepository} documentRepository
 * @param {WebAssembly.Instance} dppWasm
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  latestBlockExecutionContext,
  identityRepository,
  dataContractRepository,
  documentRepository,
  dppWasm,
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
    const timeMs = latestBlockExecutionContext.getTimeMs();
    const version = latestBlockExecutionContext.getVersion();
    const {
      quorumHash,
      blockSignature: signature,
    } = latestBlockExecutionContext.getLastCommitInfo();
    const round = latestBlockExecutionContext.getRound();

    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
      metadata: {
        height: blockHeight.toNumber(),
        coreChainLockedHeight,
        timeMs,
        protocolVersion: version.app.toNumber(),
      },
    };

    if (documents && documents.length) {
      const documentsProof = await documentRepository
        .proveManyDocumentsFromDifferentContracts(documents);

      response.documentsProof = {
        quorumHash,
        signature,
        merkleProof: documentsProof.getValue(),
        round,
      };
    }

    if (identityIds && identityIds.length) {
      const identitiesProof = await identityRepository.proveMany(
        identityIds.map((identityId) => dppWasm.Identifier.from(identityId).toBuffer()),
      );

      response.identitiesProof = {
        quorumHash,
        signature,
        merkleProof: identitiesProof.getValue(),
        round,
      };
    }

    if (dataContractIds && dataContractIds.length) {
      const dataContractsProof = await dataContractRepository.proveMany(
        dataContractIds.map((dataContractId) => dppWasm.Identifier.from(dataContractId).toBuffer()),
      );

      response.dataContractsProof = {
        quorumHash,
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
