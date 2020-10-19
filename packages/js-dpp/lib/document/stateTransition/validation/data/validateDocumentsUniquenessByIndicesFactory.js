const ValidationResult = require('../../../../validation/ValidationResult');
const DuplicateDocumentError = require('../../../../errors/DuplicateDocumentError');
const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

/**
 * @param {StateRepository} stateRepository
 * @return {validateDocumentsUniquenessByIndices}
 */
function validateDocumentsUniquenessByIndicesFactory(stateRepository) {
  /**
   * @typedef validateDocumentsUniquenessByIndices
   * @param {string} ownerId
   * @param {DocumentCreateTransition[]
   *         |DocumentReplaceTransition[]} documentTransitions
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  async function validateDocumentsUniquenessByIndices(
    ownerId,
    documentTransitions,
    dataContract,
  ) {
    const result = new ValidationResult();

    // 1. Prepare fetchDocuments queries from indexed properties
    const documentIndexQueries = documentTransitions
      .reduce((queries, documentTransition) => {
        const documentSchema = dataContract.getDocumentSchema(documentTransition.getType());

        if (!documentSchema.indices) {
          return queries;
        }

        documentSchema.indices
          .filter((index) => index.unique)
          .forEach((indexDefinition) => {
            const where = [];

            indexDefinition.properties.forEach((property) => {
              const propertyName = Object.keys(property)[0];
              let propertyValue;

              switch (propertyName) {
                case '$ownerId':
                  propertyValue = ownerId;
                  break;
                case '$createdAt':
                  if (documentTransition.getAction()
                    === AbstractDocumentTransition.ACTIONS.CREATE) {
                    const createdAt = documentTransition.getCreatedAt();
                    if (createdAt) {
                      propertyValue = createdAt.getTime();
                    }
                  }
                  break;
                case '$updatedAt': {
                  const updatedAt = documentTransition.getUpdatedAt();
                  if (updatedAt) {
                    propertyValue = updatedAt.getTime();
                  }
                }
                  break;
                default:
                  propertyValue = documentTransition.get(propertyName);
              }

              if (propertyValue !== undefined) {
                where.push([propertyName, '==', propertyValue]);
              }
            });

            queries.push({
              type: documentTransition.getType(),
              indexDefinition,
              documentTransition,
              where,
            });
          });

        return queries;
      }, []);

    // 2. Fetch Document by indexed properties
    const fetchRawDocumentPromises = documentIndexQueries
      .filter(({ where }) => where.length > 0)
      .map(async ({
        type,
        where,
        indexDefinition,
        documentTransition,
      }) => {
        const doc = await stateRepository.fetchDocuments(
          dataContract.getId(),
          type,
          { where },
        );

        return Object.assign(doc, {
          indexDefinition,
          documentTransition,
        });
      });

    let fetchedDocumentsByIndices;

    try {
      fetchedDocumentsByIndices = await Promise.all(fetchRawDocumentPromises);
    } catch (e) {
      result.addError(e);
      return result;
    }

    // 3. Create errors if duplicates found
    fetchedDocumentsByIndices
      .filter((docs) => {
        const isEmpty = docs.length === 0;
        const onlyOriginDocument = docs.length === 1
          && docs[0].getId().equals(docs.documentTransition.getId());

        return !isEmpty && !onlyOriginDocument;
      }).forEach((rawDocuments) => {
        result.addError(
          new DuplicateDocumentError(
            rawDocuments.documentTransition,
            rawDocuments.indexDefinition,
          ),
        );
      });

    return result;
  }

  return validateDocumentsUniquenessByIndices;
}

module.exports = validateDocumentsUniquenessByIndicesFactory;
