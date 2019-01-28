const Revisions = require('../revisions/Revisions');

/**
 * @param {string} contractId
 * @param {string} userId
 * @param {DPContract} dpContract
 * @param {Reference} reference
 * @param {boolean} [isDeleted=false]
 * @param {array} [previousRevisions=[]]
 */
class SVContract extends Revisions {
  constructor(
    contractId,
    userId,
    dpContract,
    reference,
    isDeleted = false,
    previousRevisions = [],
  ) {
    super(reference, previousRevisions);

    this.contractId = contractId;
    this.userId = userId;
    this.dpContract = dpContract;
    this.deleted = isDeleted;
  }

  /**
   * Get DP Contract ID
   *
   * @return {string}
   */
  getContractId() {
    return this.contractId;
  }

  /**
   * Get user ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
  }

  /**
   * Is object deleted?
   *
   * @return {boolean}
   */
  isDeleted() {
    return this.deleted;
  }

  /**
   * Mark object as deleted
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
   *            stPacketHash: string,
   *            hash: string
   *          },
   *          isDeleted: boolean,
   *          userId: string,
   *          contractId: string,
   *          previousRevisions: {
   *            revision: number,
   *            reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHash: string,
   *              stPacketHash: string,
   *              hash: string
   *            }
   *          }[],
   *          dpContract: {
   *            $schema: string,
   *            name: string,
   *            version: number,
   *            dpObjectsDefinition: Object<string, Object>,
   *            definitions?: Object<string, Object>}
   *          }}
   */
  toJSON() {
    return {
      contractId: this.getContractId(),
      userId: this.getUserId(),
      reference: this.reference.toJSON(),
      dpContract: this.getDPContract().toJSON(),
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
    return this.getDPContract().getVersion();
  }
}

module.exports = SVContract;
