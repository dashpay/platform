/**
 *
 * @param {connectToMongoDB} connectToDocumentMongoDB
 * @param {string} documentMongoDBPrefix
 * @return {getDocumentMongoDbDatabase}
 */
function getDocumentMongoDbDatabaseFactory(connectToDocumentMongoDB, documentMongoDBPrefix) {
  /**
   * @typedef getDocumentMongoDbDatabase
   * @param {Identifier} dataContractId
   * @return {Db}
   */
  async function getDocumentMongoDbDatabase(dataContractId) {
    const documentMongoDBClient = await connectToDocumentMongoDB();
    return documentMongoDBClient.db(`${documentMongoDBPrefix}${dataContractId.toString()}`);
  }

  return getDocumentMongoDbDatabase;
}

module.exports = getDocumentMongoDbDatabaseFactory;
