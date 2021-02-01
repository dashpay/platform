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
 * @return {getProofsQueryHandler}
 */
function getProofsQueryHandlerFactory(
  previousRootTree,
  previousDocumentsStoreRootTreeLeaf,
  previousIdentitiesStoreRootTreeLeaf,
  previousDataContractsStoreRootTreeLeaf,
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
    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
    };

    if (documentIds && documentIds.length) {
      response.documentsProof = previousRootTree
        .getFullProof(previousDocumentsStoreRootTreeLeaf, documentIds);
    }

    if (identityIds && identityIds.length) {
      response.identitiesProof = previousRootTree
        .getFullProof(previousIdentitiesStoreRootTreeLeaf, identityIds);
    }

    if (dataContractIds && dataContractIds.length) {
      response.dataContractsProof = previousRootTree
        .getFullProof(previousDataContractsStoreRootTreeLeaf, dataContractIds);
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
