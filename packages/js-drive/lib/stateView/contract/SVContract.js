const Revisions = require('../revisions/Revisions');

/**
 * @param {DataContract} dataContract
 * @param {Reference} reference
 * @param {boolean} [isDeleted=false]
 * @param {array} [previousRevisions=[]]
 */
class SVContract extends Revisions {
  constructor(
    dataContract,
    reference,
    isDeleted = false,
    previousRevisions = [],
  ) {
    super(reference, previousRevisions);

    this.dataContract = dataContract;
    this.deleted = isDeleted;
  }

  /**
   * Get contract id
   *
   * @return {string}
   */
  getId() {
    return this.getDataContract().getId();
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * Is contract deleted?
   *
   * @return {boolean}
   */
  isDeleted() {
    return this.deleted;
  }

  /**
   * Mark contract as deleted
   *
   * @return {SVContract}
   */
  markAsDeleted() {
    this.deleted = true;

    return this;
  }

  /**
   * Return SV Contract as plain object
   *
   * @return {{reference: {
   *            blockHash: string,
   *            blockHeight: number,
   *            stHash: string,
   *            hash: string
   *          },
   *          isDeleted: boolean,
   *          contractId: string,
   *          previousRevisions: {
   *            revision: number,
   *            reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHash: string,
   *              hash: string
   *            }
   *          }[],
   *          contract: RawContract
   *          }}
   */
  toJSON() {
    return {
      contractId: this.getId(),
      reference: this.reference.toJSON(),
      contract: this.getDataContract().toJSON(),
      isDeleted: this.isDeleted(),
      previousRevisions: this.getPreviousRevisions().map(r => r.toJSON()),
    };
  }

  /**
   * Get revision number
   *
   * @private
   * @return {number}
   */
  getRevisionNumber() {
    return this.getDataContract().getVersion();
  }
}

module.exports = SVContract;
