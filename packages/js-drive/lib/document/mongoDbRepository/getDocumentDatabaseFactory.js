/**
 *
 * @param {connectToMongoDB} connectToDocumentMongoDB
 * @param {string} documentMongoDBPrefix
 * @return {getDocumentDatabase}
 */
function getDocumentDatabaseFactory(connectToDocumentMongoDB, documentMongoDBPrefix) {
  /**
   * @typedef getDocumentDatabase
   * @param {Identifier} dataContractId
   * @return {Db}
   */
  async function getDocumentDatabase(dataContractId) {
    const documentMongoDBClient = await connectToDocumentMongoDB();
    return documentMongoDBClient.db(`${documentMongoDBPrefix}${dataContractId.toString()}`);
  }

  return getDocumentDatabase;
}

module.exports = getDocumentDatabaseFactory;
