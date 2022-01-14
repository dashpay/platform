const DataContractStoreRepository = require('../../dataContract/DataContractStoreRepository');
/**
 * @param {DataContract} dataContract
 * @param {string} documentType
 * @return {Buffer[]}
 */
function createDocumentTypeTreePath(dataContract, documentType) {
  return DataContractStoreRepository.TREE_PATH.concat([
    dataContract.getId().toBuffer(),
    documentType,
  ]);
}

module.exports = createDocumentTypeTreePath;
