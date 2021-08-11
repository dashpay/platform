const ConsensusError = require('./ConsensusError');

const AbstractDocumentTransition = require('../document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

class DocumentNotFoundError extends ConsensusError {
  /**
   * @param {DocumentReplaceTransition
   *        |DocumentDeleteTransition} documentTransition
   */
  constructor(documentTransition) {
    const noun = {
      [AbstractDocumentTransition.ACTIONS.REPLACE]: 'Updated',
      [AbstractDocumentTransition.ACTIONS.DELETE]: 'Deleted',
    };

    super(`${noun[documentTransition.getAction()]} Document not found`);

    this.documentTransition = documentTransition;
  }

  /**
   * Get Document Transition
   *
   * @return {DocumentReplaceTransition|DocumentDeleteTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = DocumentNotFoundError;
