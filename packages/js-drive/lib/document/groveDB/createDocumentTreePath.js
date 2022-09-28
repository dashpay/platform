const DataContractStoreRepository = require('../../dataContract/DataContractStoreRepository');
/**
 * @param {DataContract} dataContract
 * @param {string} documentType
 * @return {Buffer[]}
 */
function createDocumentTypeTreePath(dataContract, documentType) {
  return DataContractStoreRepository.TREE_PATH.concat([
    dataContract.getId().toBuffer(),
    Buffer.from([1]),
    Buffer.from(documentType),
  ]);
}

module.exports = createDocumentTypeTreePath;
