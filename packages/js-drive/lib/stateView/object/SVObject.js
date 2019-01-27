const Revisions = require('../revisions/Revisions');

class SVObject extends Revisions {
  /**
   * @param {string} userId
   * @param {DPObject} dpObject
   * @param {Reference} reference
   * @param {boolean} [isDeleted]
   * @param {array} [previousRevisions]
   */
  constructor(userId, dpObject, reference, isDeleted = false, previousRevisions = []) {
    super(reference, previousRevisions);

    this.userId = userId;
    this.dpObject = dpObject;
    this.deleted = isDeleted;
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
   * Get DP Object
   *
   * @return {DPObject}
   */
  getDPObject() {
    return this.dpObject;
  }

  /**
   * Mark object as deleted
   *
   * @return {SVObject}
   */
  markAsDeleted() {
    this.deleted = true;

    return this;
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
   * Get revision number
   *
   * @private
   * @return {number}
   */
  getRevisionNumber() {
    return this.getDPObject().getRevision();
  }

  /**
   * Return SV Object as plain object
   *
   * @return {{reference: {
   *            blockHash: string,
   *            blockHeight: number,
   *            stHeaderHash: string,
   *            stPacketHash: string,
   *            hash: string
   *           },
   *           isDeleted: boolean,
   *           userId: string,
   *           dpObject: { $scope, $action, $scopeId, $rev, $type },
   *           previousRevisions: {
   *            revision: number,
   *            reference: {
   *              blockHash: string,
   *              blockHeight: number,
   *              stHeaderHash: string,
   *              stPacketHash: string,
   *              hash: string
   *            }
   *           }[]}}
   */
  toJSON() {
    return {
      userId: this.getUserId(),
      isDeleted: this.isDeleted(),
      dpObject: this.getDPObject().toJSON(),
      reference: this.getReference().toJSON(),
      previousRevisions: this.getPreviousRevisions().map(r => r.toJSON()),
    };
  }
}

module.exports = SVObject;
