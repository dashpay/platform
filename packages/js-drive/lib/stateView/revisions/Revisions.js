const Revision = require('./Revision');

class Revisions {
  /**
   * @param {Reference} reference
   * @param {Revisions[]} previousRevisions
   */
  constructor(reference, previousRevisions) {
    this.reference = reference;
    this.previousRevisions = previousRevisions;
  }

  /**
   * Get revision number
   *
   * @abstract
   * @private
   * @return {number}
   */
  getRevisionNumber() {
    throw new Error('Method should be overloaded in child class');
  }

  /**
   * Get reference
   *
   * @return {Reference}
   */
  getReference() {
    return this.reference;
  }

  /**
   * Get current revision
   *
   * @return {Revision}
   */
  getCurrentRevision() {
    return new Revision(
      this.getRevisionNumber(),
      this.getReference(),
    );
  }

  /**
   * Add revision
   *
   * @param {Revisions} revisions
   *
   * @return {Revisions}
   */
  addRevision(revisions) {
    this.previousRevisions = this.previousRevisions
      .concat(revisions.getPreviousRevisions())
      .concat([revisions.getCurrentRevision()]);

    return this;
  }

  /**
   * Get previous revisions
   *
   * @return {Revision[]}
   */
  getPreviousRevisions() {
    return this.previousRevisions;
  }

  /**
   * Remove revisions which are higher than current document revision
   *
   * @return {Revisions}
   */
  removeAheadRevisions() {
    this.previousRevisions = this.previousRevisions
      .filter(({ revision }) => revision < this.getRevisionNumber());

    return this;
  }
}

module.exports = Revisions;
