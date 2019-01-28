class Revision {
  /**
   * @param {number} revision
   * @param {Reference} reference
   */
  constructor(revision, reference) {
    this.revision = revision;
    this.reference = reference;
  }

  /**
   * Get revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
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
   * Get Revision as plain object
   *
   * @return {{ reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHash: string,
   *              stPacketHash: string,
   *              hash: string
   *            },
   *            revision: number}}
   */
  toJSON() {
    return {
      revision: this.getRevision(),
      reference: this.getReference().toJSON(),
    };
  }
}

module.exports = Revision;
