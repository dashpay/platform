const DataTriggerExecutionContext = require('../../../../dataTrigger/DataTriggerExecutionContext');

const ValidationResult = require('../../../../validation/ValidationResult');

const DocumentAlreadyPresentError = require('../../../../errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../errors/DocumentNotFoundError');
const DocumentOwnerIdMismatchError = require('../../../../errors/DocumentOwnerIdMismatchError');
const InvalidDocumentRevisionError = require('../../../../errors/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../errors/InvalidDocumentActionError');
const DataContractNotPresentError = require('../../../../errors/DataContractNotPresentError');
const DocumentTimestampWindowViolationError = require(
  '../../../../errors/DocumentTimestampWindowViolationError',
);
const DocumentTimestampsMismatchError = require(
  '../../../../errors/DocumentTimestampsMismatchError',
);

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

const BLOCK_TIME_WINDOW_MINUTES = 5;

/**
 *
 * @param {StateRepository} stateRepository
 * @param {fetchDocuments} fetchDocuments
 * @param {validateDocumentsUniquenessByIndices} validateDocumentsUniquenessByIndices
 * @param {validatePartialCompoundIndices} validatePartialCompoundIndices
 * @param {executeDataTriggers} executeDataTriggers
 * @return {validateDocumentsBatchTransitionData}
 */
function validateDocumentsBatchTransitionDataFactory(
  stateRepository,
  fetchDocuments,
  validateDocumentsUniquenessByIndices,
  validatePartialCompoundIndices,
  executeDataTriggers,
) {
  /**
   *
   * @param {Buffer} dataContractId
   * @param {Buffer} ownerId
   * @param {AbstractDocumentTransition} documentTransitions
   * @return {Promise<ValidationResult>}
   */
  async function validateDocumentTransitions(dataContractId, ownerId, documentTransitions) {
    const result = new ValidationResult();

    // Data contract must exist
    const dataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const fetchedDocuments = await fetchDocuments(documentTransitions);

    // Calculate time window for timestamps
    const {
      time: {
        seconds: lastBlockHeaderTimeSeconds,
      },
    } = await stateRepository.fetchLatestPlatformBlockHeader();

    // Get last block header time in milliseconds
    const lastBlockHeaderTime = lastBlockHeaderTimeSeconds * 1000;

    // Define time window
    const timeWindowStart = new Date(lastBlockHeaderTime);
    timeWindowStart.setMinutes(
      timeWindowStart.getMinutes() - BLOCK_TIME_WINDOW_MINUTES,
    );

    const timeWindowEnd = new Date(lastBlockHeaderTime);
    timeWindowEnd.setMinutes(
      timeWindowEnd.getMinutes() + BLOCK_TIME_WINDOW_MINUTES,
    );

    // Validate document action, ownerId, revision and timestamps
    documentTransitions
      .forEach((documentTransition) => {
        const fetchedDocument = fetchedDocuments
          .find((d) => documentTransition.getId().equals(d.getId()));

        switch (documentTransition.getAction()) {
          case AbstractDocumentTransition.ACTIONS.CREATE:
            // createdAt and updatedAt should be equal
            if (documentTransition.getCreatedAt() !== undefined
                && documentTransition.getUpdatedAt() !== undefined) {
              const createdAtTime = documentTransition.getCreatedAt().getTime();
              const updatedAtTime = documentTransition.getUpdatedAt().getTime();

              if (createdAtTime !== updatedAtTime) {
                result.addError(new DocumentTimestampsMismatchError(documentTransition));
              }
            }

            // Check createdAt is within a block time window
            if (documentTransition.getCreatedAt() !== undefined) {
              const createdAtTime = documentTransition.getCreatedAt().getTime();

              if (createdAtTime < timeWindowStart || createdAtTime > timeWindowEnd) {
                result.addError(
                  new DocumentTimestampWindowViolationError(
                    'createdAt', documentTransition, fetchedDocument,
                  ),
                );
              }
            }

            // Check updatedAt is within a block time window
            if (documentTransition.getUpdatedAt() !== undefined) {
              const updatedAtTime = documentTransition.getUpdatedAt().getTime();

              if (updatedAtTime < timeWindowStart || updatedAtTime > timeWindowEnd) {
                result.addError(
                  new DocumentTimestampWindowViolationError(
                    'updatedAt', documentTransition, fetchedDocument,
                  ),
                );
              }
            }

            if (fetchedDocument) {
              result.addError(
                new DocumentAlreadyPresentError(documentTransition, fetchedDocument),
              );
            }
            break;
          case AbstractDocumentTransition.ACTIONS.REPLACE: {
            // Check updatedAt is within a block time window
            if (documentTransition.getUpdatedAt() !== undefined) {
              const updatedAtTime = documentTransition.getUpdatedAt().getTime();

              if (updatedAtTime < timeWindowStart || updatedAtTime > timeWindowEnd) {
                result.addError(
                  new DocumentTimestampWindowViolationError(
                    'updatedAt', documentTransition, fetchedDocument,
                  ),
                );
              }
            }

            if (
              fetchedDocument
              && documentTransition.getRevision() !== fetchedDocument.getRevision() + 1
            ) {
              result.addError(
                new InvalidDocumentRevisionError(documentTransition, fetchedDocument),
              );
            }
          }
          // eslint-disable-next-line no-fallthrough
          case AbstractDocumentTransition.ACTIONS.DELETE: {
            if (!fetchedDocument) {
              result.addError(
                new DocumentNotFoundError(documentTransition),
              );

              break;
            }

            if (!fetchedDocument.getOwnerId().equals(ownerId)) {
              result.addError(
                new DocumentOwnerIdMismatchError(documentTransition, fetchedDocument),
              );
            }

            break;
          }
          default:
            throw new InvalidDocumentActionError(documentTransition);
        }
      });

    if (!result.isValid()) {
      return result;
    }

    // Validate unique indices
    const nonDeleteDocumentTransitions = documentTransitions
      .filter((d) => d.getAction() !== AbstractDocumentTransition.ACTIONS.DELETE);

    if (nonDeleteDocumentTransitions.length > 0) {
      result.merge(
        await validateDocumentsUniquenessByIndices(
          ownerId,
          nonDeleteDocumentTransitions,
          dataContract,
        ),
      );

      result.merge(
        validatePartialCompoundIndices(
          ownerId,
          nonDeleteDocumentTransitions,
          dataContract,
        ),
      );

      if (!result.isValid()) {
        return result;
      }
    }

    // Run Data Triggers
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepository,
      ownerId,
      dataContract,
    );

    const dataTriggersExecutionResults = await executeDataTriggers(
      documentTransitions,
      dataTriggersExecutionContext,
    );

    dataTriggersExecutionResults.forEach((dataTriggerExecutionResult) => {
      if (!dataTriggerExecutionResult.isOk()) {
        result.addError(...dataTriggerExecutionResult.getErrors());
      }
    });

    return result;
  }
  /**
   * @typedef validateDocumentsBatchTransitionData
   * @param {DocumentsBatchTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionData(stateTransition) {
    const result = new ValidationResult();

    const ownerId = stateTransition.getOwnerId();

    // Group document transitions by data contracts
    const documentTransitionsByContracts = stateTransition.getTransitions()
      .reduce((obj, documentTransition) => {
        if (!obj[documentTransition.getDataContractId()]) {
          // eslint-disable-next-line no-param-reassign
          obj[documentTransition.getDataContractId()] = {
            dataContractId: documentTransition.getDataContractId(),
            documentTransitions: [],
          };
        }

        obj[documentTransition.getDataContractId()].documentTransitions.push(documentTransition);

        return obj;
      }, {});

    const documentTransitionResultsPromises = Object.entries(documentTransitionsByContracts)
      .map(([, { dataContractId, documentTransitions }]) => (
        validateDocumentTransitions(dataContractId, ownerId, documentTransitions)
      ));

    const documentTransitionResults = await Promise.all(documentTransitionResultsPromises);
    documentTransitionResults.forEach(result.merge.bind(result));

    return result;
  }

  return validateDocumentsBatchTransitionData;
}

module.exports = validateDocumentsBatchTransitionDataFactory;
