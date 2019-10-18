const Document = require('../../../Document');

const DataTriggerExecutionContext = require('../../../../dataTrigger/DataTriggerExecutionContext');

const ValidationResult = require('../../../../validation/ValidationResult');

const DataContractNotPresentError = require('../../../../errors/DataContractNotPresentError');
const DocumentAlreadyPresentError = require('../../../../errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../errors/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../../stPacket/errors/InvalidDocumentActionError');

/**
 *
 * @param {DataProvider} dataProvider
 * @param {validateBlockchainUser} validateBlockchainUser
 * @param {fetchDocuments} fetchDocuments
 * @param {validateDocumentsUniquenessByIndices} validateDocumentsUniquenessByIndices
 * @param {executeDataTriggers} executeDataTriggers
 * @return {validateDataContractSTData}
 */
function validateDocumentsSTDataFactory(
  dataProvider,
  validateBlockchainUser,
  fetchDocuments,
  validateDocumentsUniquenessByIndices,
  executeDataTriggers,
) {
  /**
   * @typedef validateDocumentsSTData
   * @param {DocumentsStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsSTData(stateTransition) {
    const result = new ValidationResult();

    const documents = stateTransition.getDocuments();
    const [firstDocument] = documents;

    const userId = firstDocument.getUserId();
    const dataContractId = firstDocument.getDataContractId();

    // User must exist and confirmed
    result.merge(
      await validateBlockchainUser(userId),
    );

    if (!result.isValid()) {
      return result;
    }

    // Data contract must exist
    const dataContract = await dataProvider.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );

      return result;
    }

    // Validate document action, userId and revision
    const fetchedDocuments = await fetchDocuments(documents);

    documents
      .forEach((document) => {
        const fetchedDocument = fetchedDocuments.find(d => document.getId() === d.getId());

        switch (document.getAction()) {
          case Document.ACTIONS.CREATE:
            if (fetchedDocument) {
              result.addError(
                new DocumentAlreadyPresentError(document, fetchedDocument),
              );
            }
            break;
          case Document.ACTIONS.REPLACE:
          case Document.ACTIONS.DELETE: {
            if (!fetchedDocument) {
              result.addError(
                new DocumentNotFoundError(document),
              );

              break;
            }

            if (document.getRevision() !== fetchedDocument.getRevision() + 1) {
              result.addError(
                new InvalidDocumentRevisionError(document, fetchedDocument),
              );
            }

            break;
          }
          default:
            throw new InvalidDocumentActionError(document);
        }
      });

    if (!result.isValid()) {
      return result;
    }

    // Validate unique indices
    result.merge(
      await validateDocumentsUniquenessByIndices(documents, dataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    // Run Data Triggers
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      dataProvider,
      userId,
      dataContract,
    );

    const dataTriggersExecutionResults = await executeDataTriggers(
      documents,
      dataTriggersExecutionContext,
    );

    dataTriggersExecutionResults.forEach((dataTriggerExecutionResult) => {
      if (!dataTriggerExecutionResult.isOk()) {
        result.addError(...dataTriggerExecutionResult.getErrors());
      }
    });

    return result;
  }

  return validateDocumentsSTData;
}

module.exports = validateDocumentsSTDataFactory;
