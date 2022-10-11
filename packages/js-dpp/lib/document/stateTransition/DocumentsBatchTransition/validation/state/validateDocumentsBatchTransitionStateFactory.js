const DataTriggerExecutionContext = require('../../../../../dataTrigger/DataTriggerExecutionContext');

const ValidationResult = require('../../../../../validation/ValidationResult');

const DocumentAlreadyPresentError = require('../../../../../errors/consensus/state/document/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../../errors/consensus/state/document/DocumentNotFoundError');
const DocumentOwnerIdMismatchError = require('../../../../../errors/consensus/state/document/DocumentOwnerIdMismatchError');
const InvalidDocumentRevisionError = require('../../../../../errors/consensus/state/document/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../../errors/InvalidDocumentActionError');
const DataContractNotPresentError = require('../../../../../errors/DataContractNotPresentError');
const DocumentTimestampWindowViolationError = require(
  '../../../../../errors/consensus/state/document/DocumentTimestampWindowViolationError',
);
const DocumentTimestampsMismatchError = require(
  '../../../../../errors/consensus/state/document/DocumentTimestampsMismatchError',
);

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

const validateTimeInBlockTimeWindow = require('../../../../../blockTimeWindow/validateTimeInBlockTimeWindow');
const StateTransitionExecutionContext = require('../../../../../stateTransition/StateTransitionExecutionContext');

/**
 *
 * @param {StateRepository} stateRepository
 * @param {fetchDocuments} fetchDocuments
 * @param {validateDocumentsUniquenessByIndices} validateDocumentsUniquenessByIndices
 * @param {executeDataTriggers} executeDataTriggers
 * @return {validateDocumentsBatchTransitionState}
 */
function validateDocumentsBatchTransitionStateFactory(
  stateRepository,
  fetchDocuments,
  validateDocumentsUniquenessByIndices,
  executeDataTriggers,
) {
  /**
   *
   * @param {Identifier} dataContractId
   * @param {Identifier} ownerId
   * @param {DocumentCreateTransition[]
   *        |DocumentReplaceTransition[]
   *        |DocumentDeleteTransition[]} documentTransitions
   * @param {StateTransitionExecutionContext} executionContext
   * @return {Promise<ValidationResult>}
   */
  async function validateDocumentTransitions(
    dataContractId,
    ownerId,
    documentTransitions,
    executionContext,
  ) {
    const result = new ValidationResult();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // data contract to proceed with following logic
    const tmpExecutionContext = new StateTransitionExecutionContext();

    // Data contract must exist
    const dataContract = await stateRepository.fetchDataContract(
      dataContractId,
      tmpExecutionContext,
    );

    // Collect operations back from temporary context
    executionContext.addOperation(...tmpExecutionContext.getOperations());

    if (!dataContract) {
      throw new DataContractNotPresentError(dataContractId);
    }

    if (!result.isValid()) {
      return result;
    }

    const fetchedDocuments = await fetchDocuments(documentTransitions, executionContext);

    if (!executionContext.isDryRun()) {
      // Calculate time window for timestamps
      const {
        time: {
          seconds: lastBlockHeaderTimeSeconds,
        },
      } = await stateRepository.fetchLatestPlatformBlockHeader();

      // Get last block header time in milliseconds
      const lastBlockHeaderTime = lastBlockHeaderTimeSeconds * 1000;

      // Validate document action, ownerId, revision and timestamps
      documentTransitions
        .forEach((documentTransition) => {
          const fetchedDocument = fetchedDocuments
            .find((d) => documentTransition.getId()
              .equals(d.getId()));

          switch (documentTransition.getAction()) {
            case AbstractDocumentTransition.ACTIONS.CREATE:
              // createdAt and updatedAt should be equal
              if (documentTransition.getCreatedAt() !== undefined
                && documentTransition.getUpdatedAt() !== undefined) {
                const createdAtTime = documentTransition.getCreatedAt()
                  .getTime();
                const updatedAtTime = documentTransition.getUpdatedAt()
                  .getTime();

                if (createdAtTime !== updatedAtTime) {
                  result.addError(
                    new DocumentTimestampsMismatchError(
                      documentTransition.getId()
                        .toBuffer(),
                    ),
                  );
                }
              }

              // Check createdAt is within a block time window
              if (documentTransition.getCreatedAt() !== undefined) {
                const createdAtTime = documentTransition.getCreatedAt()
                  .getTime();

                const validateTimeWindowResult = validateTimeInBlockTimeWindow(
                  lastBlockHeaderTime,
                  createdAtTime,
                );
                if (!validateTimeWindowResult.isValid()) {
                  result.addError(
                    new DocumentTimestampWindowViolationError(
                      'createdAt',
                      documentTransition.getId()
                        .toBuffer(),
                      documentTransition.getCreatedAt(),
                      validateTimeWindowResult.getTimeWindowStart(),
                      validateTimeWindowResult.getTimeWindowEnd(),
                    ),
                  );
                }
              }

              // Check updatedAt is within a block time window
              if (documentTransition.getUpdatedAt() !== undefined) {
                const updatedAtTime = documentTransition.getUpdatedAt()
                  .getTime();
                const validateTimeWindowResult = validateTimeInBlockTimeWindow(
                  lastBlockHeaderTime,
                  updatedAtTime,
                );

                if (!validateTimeWindowResult.isValid()) {
                  result.addError(
                    new DocumentTimestampWindowViolationError(
                      'updatedAt',
                      documentTransition.getId()
                        .toBuffer(),
                      documentTransition.getUpdatedAt(),
                      validateTimeWindowResult.getTimeWindowStart(),
                      validateTimeWindowResult.getTimeWindowEnd(),
                    ),
                  );
                }
              }

              if (fetchedDocument) {
                result.addError(
                  new DocumentAlreadyPresentError(documentTransition.getId()
                    .toBuffer()),
                );
              }
              break;
            case AbstractDocumentTransition.ACTIONS.REPLACE: {
              // Check updatedAt is within a block time window
              if (documentTransition.getUpdatedAt() !== undefined) {
                const updatedAtTime = documentTransition.getUpdatedAt()
                  .getTime();

                const validateTimeWindowResult = validateTimeInBlockTimeWindow(
                  lastBlockHeaderTime,
                  updatedAtTime,
                );

                if (!validateTimeWindowResult.isValid()) {
                  result.addError(
                    new DocumentTimestampWindowViolationError(
                      'updatedAt',
                      documentTransition.getId()
                        .toBuffer(),
                      documentTransition.getUpdatedAt(),
                      validateTimeWindowResult.getTimeWindowStart(),
                      validateTimeWindowResult.getTimeWindowEnd(),
                    ),
                  );
                }
              }

              if (
                fetchedDocument
                && documentTransition.getRevision() !== fetchedDocument.getRevision() + 1
              ) {
                result.addError(
                  new InvalidDocumentRevisionError(
                    documentTransition.getId()
                      .toBuffer(),
                    fetchedDocument.getRevision(),
                  ),
                );
              }
            }
            // eslint-disable-next-line no-fallthrough
            case AbstractDocumentTransition.ACTIONS.DELETE: {
              if (!fetchedDocument) {
                result.addError(
                  new DocumentNotFoundError(documentTransition.getId()
                    .toBuffer()),
                );

                break;
              }

              if (!fetchedDocument.getOwnerId()
                .equals(ownerId)) {
                result.addError(
                  new DocumentOwnerIdMismatchError(
                    documentTransition.getId()
                      .toBuffer(),
                    ownerId.toBuffer(),
                    fetchedDocument.getOwnerId()
                      .toBuffer(),
                  ),
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
          executionContext,
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
      executionContext,
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
   * @typedef validateDocumentsBatchTransitionState
   * @param {DocumentsBatchTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionState(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();
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
        validateDocumentTransitions(dataContractId, ownerId, documentTransitions, executionContext)
      ));

    const documentTransitionResults = await Promise.all(documentTransitionResultsPromises);
    documentTransitionResults.forEach(result.merge.bind(result));

    return result;
  }

  return validateDocumentsBatchTransitionState;
}

module.exports = validateDocumentsBatchTransitionStateFactory;
