const ValidationResult = require('../../../../validation/ValidationResult');
const DuplicateDocumentError = require('../../../../errors/DuplicateDocumentError');

/**
 * @param {DataProvider} dataProvider
 * @return {validateDocumentsUniquenessByIndices}
 */
function validateDocumentsUniquenessByIndicesFactory(dataProvider) {
  /**
   * @typedef validateDocumentsUniquenessByIndices
   * @param {Document[]} documents
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  async function validateDocumentsUniquenessByIndices(documents, dataContract) {
    const result = new ValidationResult();

    const userId = documents[0].getUserId();

    // 1. Prepare fetchDocuments queries from indexed properties
    const documentIndexQueries = documents
      .reduce((queries, document) => {
        const documentSchema = dataContract.getDocumentSchema(document.getType());

        if (!documentSchema.indices) {
          return queries;
        }

        documentSchema.indices
          .filter((index) => index.unique)
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
    const fetchRawDocumentPromises = documentIndexQueries
      .map(({
        type,
        where,
        indexDefinition,
        originDocument,
      }) => (
        dataProvider.fetchDocuments(
          dataContract.getId(),
          type,
          { where },
        )
          .then((doc) => Object.assign(doc, {
            indexDefinition,
            originDocument,
          }))
      ));

    const fetchedRawDocumentsByIndices = await Promise.all(fetchRawDocumentPromises);

    // 3. Create errors if duplicates found
    fetchedRawDocumentsByIndices
      .filter((docs) => {
        const isEmpty = docs.length === 0;
        const onlyOriginDocument = docs.length === 1
          && docs[0].getId() === docs.originDocument.getId();

        return !isEmpty && !onlyOriginDocument;
      }).forEach((rawDocuments) => {
        result.addError(
          new DuplicateDocumentError(
            rawDocuments.originDocument,
            rawDocuments.indexDefinition,
          ),
        );
      });

    return result;
  }

  return validateDocumentsUniquenessByIndices;
}

module.exports = validateDocumentsUniquenessByIndicesFactory;
