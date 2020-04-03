/**
 *
 * @param {MongoClient} documentMongoDBClient
 * @param {string} documentMongoDBPrefix
 * @return {getDocumentDatabase}
 */
function getDocumentDatabaseFactory(documentMongoDBClient, documentMongoDBPrefix) {
  /**
   * @typedef getDocumentDatabase
   * @param {string} dataContractId
   * @return {Db}
   */
  function getDocumentDatabase(dataContractId) {
    return documentMongoDBClient.db(`${documentMongoDBPrefix}${dataContractId}`);
  }

  return getDocumentDatabase;
}

module.exports = getDocumentDatabaseFactory;
