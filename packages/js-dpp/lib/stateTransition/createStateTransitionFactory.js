const types = require('./stateTransitionTypes');

const Document = require('../document/Document');

const DocumentsStateTransition = require('../document/stateTransition/DocumentsStateTransition');
const DataContractStateTransition = require('../dataContract/stateTransition/DataContractStateTransition');

const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');

/**
 * @param {createDataContract} createDataContract
 * @return {createStateTransition}
 */
function createStateTransitionFactory(createDataContract) {
  /**
   * @typedef createStateTransition
   * @param {RawDataContractStateTransition|RawDocumentsStateTransition} rawStateTransition
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  function createStateTransition(rawStateTransition) {
    switch (rawStateTransition.type) {
      case types.DATA_CONTRACT: {
        const dataContract = createDataContract(rawStateTransition.dataContract);

        return new DataContractStateTransition(dataContract);
      }
      case types.DOCUMENTS: {
        const documents = rawStateTransition.documents.map((rawDocument, index) => {
          const document = new Document(rawDocument);

          document.setAction(rawStateTransition.actions[index]);

          return document;
        });

        return new DocumentsStateTransition(documents);
      }
      default:
        throw new InvalidStateTransitionTypeError(rawStateTransition);
    }
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
