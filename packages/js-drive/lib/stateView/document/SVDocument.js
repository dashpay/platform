const Revisions = require('../revisions/Revisions');

class SVDocument extends Revisions {
  /**
   * @param {Document} document
   * @param {Reference} reference
   * @param {boolean} [isDeleted]
   * @param {array} [previousRevisions]
   */
  constructor(document, reference, isDeleted = false, previousRevisions = []) {
    super(reference, previousRevisions);

    this.document = document;
    this.deleted = isDeleted;
  }

  /**
   * Get document user id
   *
   * @return {string}
   */
  getUserId() {
    return this.document.getUserId();
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }

  /**
   * Mark document as deleted
   *
   * @return {SVDocument}
   */
  markAsDeleted() {
    this.deleted = true;

    return this;
  }

  /**
   * Is document deleted?
   *
   * @return {boolean}
   */
  isDeleted() {
    return this.deleted;
  }

  /**
   * Get revision number
   *
   * @private
   * @return {number}
   */
  getRevisionNumber() {
    return this.getDocument().getRevision();
  }

  /**
   * Return SVDocument as plain object
   *
   * @return {{reference: {
   *            blockHash: string,
   *            blockHeight: number,
   *            stHash: string,
   *            hash: string
   *           },
   *           isDeleted: boolean,
   *           userId: string,
   *           contractId: string,
   *           data: RawDocument,
   *           entropy: string,
   *           action: number,
   *           currentRevision: {
   *            revision: number,
   *            reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHash: string,
   *              hash: string
   *            }
   *           },
   *           previousRevisions: {
   *            revision: number,
   *            reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHash: string,
   *              hash: string
   *            }
   *           }[]}}
   */
  toJSON() {
    return {
      userId: this.getUserId(),
      contractId: this.getDocument().getDataContractId(),
      isDeleted: this.isDeleted(),
      data: this.getDocument().getData(),
      reference: this.getReference().toJSON(),
      entropy: this.getDocument().entropy,
      action: this.getDocument().getAction(),
      currentRevision: this.getCurrentRevision().toJSON(),
      previousRevisions: this.getPreviousRevisions().map(r => r.toJSON()),
    };
  }
}

module.exports = SVDocument;
