const DPPError = require('../../errors/DPPError');

class InvalidInitialRevisionError extends DPPError {
  /**
   * @param {Document} document
   */
  constructor(document) {
    super(`Invalid Document initial revision ${document.getRevision()}`);

    this.document = document;
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = InvalidInitialRevisionError;
