/* eslint-disable no-await-in-loop */
const AbstractDocumentTransition = require(
  './documentTransition/AbstractDocumentTransition',
);

const InvalidDocumentActionError = require('../../errors/InvalidDocumentActionError');

const Document = require('../../Document');
const DocumentNotProvidedError = require('../../errors/DocumentNotProvidedError');

/**
 * @param {StateRepository} stateRepository
 * @param {fetchDocuments} fetchDocuments
 *
 * @returns {applyDocumentsBatchTransition}
 */
function applyDocumentsBatchTransitionFactory(
  stateRepository,
  fetchDocuments,
) {
  /**
   * Apply documents batch state transition
   *
   * @typedef applyDocumentsBatchTransition
   *
   * @param {DocumentsBatchTransition} stateTransition
   *
   * @return {Promise<*>}
   */
  async function applyDocumentsBatchTransition(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    // Fetch documents for replace transitions
    const replaceTransitions = stateTransition.getTransitions()
      .filter((dt) => dt.getAction() === AbstractDocumentTransition.ACTIONS.REPLACE);

    const fetchedDocuments = await fetchDocuments(replaceTransitions, executionContext);

    const fetchedDocumentsById = fetchedDocuments.reduce((result, document) => (
      {
        ...result,
        [document.getId()]: document,
      }
    ), {});

    // since groveDB doesn't support parallel inserts, wee need to make them sequential
    // we don't want to use regenerator-runtime, so we use `reduce` instead
    // more info at https://jrsinclair.com/articles/2019/how-to-run-async-js-in-parallel-or-sequential/

    const starterPromise = Promise.resolve(null);

    return stateTransition.getTransitions().reduce(
      (previousPromise, documentTransition) => previousPromise.then(async () => {
        switch (documentTransition.getAction()) {
          case AbstractDocumentTransition.ACTIONS.CREATE: {
            const newDocument = new Document({
              $protocolVersion: stateTransition.getProtocolVersion(),
              $id: documentTransition.getId(),
              $type: documentTransition.getType(),
              $dataContractId: documentTransition.getDataContractId(),
              $ownerId: stateTransition.getOwnerId(),
              ...documentTransition.getData(),
            }, documentTransition.getDataContract());

            if (documentTransition.getCreatedAt()) {
              newDocument.setCreatedAt(documentTransition.getCreatedAt());
            }

            if (documentTransition.getUpdatedAt()) {
              newDocument.setUpdatedAt(documentTransition.getUpdatedAt());
            }

            newDocument.setEntropy(documentTransition.getEntropy());

            newDocument.setRevision(documentTransition.getRevision());

            return stateRepository.createDocument(newDocument, executionContext);
          }
          case AbstractDocumentTransition.ACTIONS.REPLACE: {
            let document;
            if (executionContext.isDryRun()) {
              const {
                time: {
                  seconds: lastBlockHeaderTimeSeconds,
                },
              } = await stateRepository.fetchLatestPlatformBlockHeader();

              const lastBlockHeaderTime = lastBlockHeaderTimeSeconds * 1000;

              document = new Document({
                $protocolVersion: stateTransition.getProtocolVersion(),
                $id: documentTransition.getId(),
                $type: documentTransition.getType(),
                $dataContractId: documentTransition.getDataContractId(),
                $ownerId: stateTransition.getOwnerId(),
                $createdAt: lastBlockHeaderTime,
                ...documentTransition.getData(),
              }, documentTransition.getDataContract());
            } else {
              document = fetchedDocumentsById[documentTransition.getId()];

              if (!document) {
                throw new DocumentNotProvidedError(documentTransition);
              }
            }

            document.setRevision(documentTransition.getRevision());
            document.setData(documentTransition.getData());

            if (documentTransition.getUpdatedAt()) {
              document.setUpdatedAt(documentTransition.getUpdatedAt());
            }

            return stateRepository.updateDocument(document, executionContext);
          }
          case AbstractDocumentTransition.ACTIONS.DELETE: {
            return stateRepository.removeDocument(
              documentTransition.getDataContract(),
              documentTransition.getType(),
              documentTransition.getId(),
              executionContext,
            );
          }
          default:
            throw new InvalidDocumentActionError(documentTransition);
        }
      }),
      starterPromise,
    );
  }

  return applyDocumentsBatchTransition;
}

module.exports = applyDocumentsBatchTransitionFactory;
