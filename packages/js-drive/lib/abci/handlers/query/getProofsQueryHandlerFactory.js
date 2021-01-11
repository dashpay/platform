const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

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
   * @param {Identifier[]} params.identityIds
   * @param {Identifier[]} params.documentIds
   * @param {Identifier[]} params.dataContractIds
   * @return {Promise<ResponseQuery>}
   */
  async function getProofsQueryHandler(params) {
    const response = {
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
    };

    if (!params) {
      return new ResponseQuery(response);
    }

    const { identityIds, documentIds, dataContractIds } = params;

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

    return new ResponseQuery(response);
  }

  return getProofsQueryHandler;
}

module.exports = getProofsQueryHandlerFactory;
