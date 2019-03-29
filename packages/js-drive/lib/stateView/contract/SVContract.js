const Revisions = require('../revisions/Revisions');

/**
 * @param {string} contractId
 * @param {string} userId
 * @param {Contract} contract
 * @param {Reference} reference
 * @param {boolean} [isDeleted=false]
 * @param {array} [previousRevisions=[]]
 */
class SVContract extends Revisions {
  constructor(
    contractId,
    userId,
    contract,
    reference,
    isDeleted = false,
    previousRevisions = [],
  ) {
    super(reference, previousRevisions);

    this.contractId = contractId;
    this.userId = userId;
    this.contract = contract;
    this.deleted = isDeleted;
  }

  /**
   * Get Contract ID
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
   * Get Contract
   *
   * @return {Contract}
   */
  getContract() {
    return this.contract;
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
   *          contract: RawContract
   *          }}
   */
  toJSON() {
    return {
      contractId: this.getContractId(),
      userId: this.getUserId(),
      reference: this.reference.toJSON(),
      contract: this.getContract().toJSON(),
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
    return this.getContract().getVersion();
  }
}

module.exports = SVContract;
