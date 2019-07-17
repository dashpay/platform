const ValidationResult = require('../../validation/ValidationResult');
const DuplicateDocumentError = require('../../errors/DuplicateDocumentError');
const Document = require('../../document/Document');

/**
 * @param {fetchDocumentsByDocuments} fetchDocumentsByDocuments
 * @param {DataProvider} dataProvider
 * @return {verifyDocumentsUniquenessByIndices}
 */
function verifyDocumentsUniquenessByIndicesFactory(fetchDocumentsByDocuments, dataProvider) {
  /**
   * @typedef verifyDocumentsUniquenessByIndices
   * @param {STPacket} stPacket
   * @param {string} userId
   * @param {Contract} contract
   * @return {ValidationResult}
   */
  async function verifyDocumentsUniquenessByIndices(stPacket, userId, contract) {
    const result = new ValidationResult();

    // 1. Prepare fetchDocuments queries from indexed properties
    const documentIndexQueries = stPacket.getDocuments()
      .reduce((queries, document) => {
        const documentSchema = contract.getDocumentSchema(document.getType());

        if (!documentSchema.indices) {
          return queries;
        }

        documentSchema.indices
          .filter(index => index.unique)
          .forEach((indexDefinition) => {
            const where = indexDefinition.properties
              .map((property) => {
                const propertyName = Object.keys(property)[0];

                let propertyValue;
                if (propertyName === '$userId') {
                  propertyValue = userId;
                } else {
                  propertyValue = document.get(propertyName);
                }

                return [propertyName, '==', propertyValue];
              });

            queries.push({
              type: document.getType(),
              indexDefinition,
              originDocument: document,
              where,
            });
          });

        return queries;
      }, []);

    // 2. Fetch Document by indexed properties
    const fetchDocumentPromises = documentIndexQueries
      .map(({
        type,
        where,
        indexDefinition,
        originDocument,
      }) => (
        dataProvider.fetchDocuments(
          stPacket.getContractId(),
          type,
          { where },
        )
          .then(documents => Object.assign(documents, {
            indexDefinition,
            originDocument,
          }))
      ));

    const fetchedDocumentsByIndices = await Promise.all(fetchDocumentPromises);

    // 3. Create errors if duplicates found
    fetchedDocumentsByIndices
      .filter((documents) => {
        const isEmpty = documents.length === 0;
        const onlyOriginDocument = documents.length === 1
          && new Document(documents[0]).getId() === documents.originDocument.getId();

        return !isEmpty && !onlyOriginDocument;
      }).forEach((documents) => {
        result.addError(
          new DuplicateDocumentError(
            documents.originDocument,
            documents.indexDefinition,
          ),
        );
      });

    return result;
  }

  return verifyDocumentsUniquenessByIndices;
}

module.exports = verifyDocumentsUniquenessByIndicesFactory;
