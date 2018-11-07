const generateDapObjectId = require('./generateDapObjectId');

class DapObject {
  /**
   * @param {string} blockchainUserId
   * @param {object} data
   * @param {Reference} reference
   * @param {boolean} isDeleted
   * @param {array} previousRevisions
   */
  constructor(blockchainUserId, data, reference, isDeleted, previousRevisions = []) {
    this.blockchainUserId = blockchainUserId;
    this.type = data.objtype;
    this.object = data;
    this.revision = data.rev;
    this.reference = reference;
    this.deleted = isDeleted;
    this.previousRevisions = previousRevisions;
  }

  getId() {
    return generateDapObjectId(this.blockchainUserId, this.object.idx);
  }

  getAction() {
    return this.object.act;
  }

  getRevision() {
    return this.revision;
  }

  getPreviousRevisions() {
    return this.previousRevisions;
  }

  isDeleted() {
    return this.deleted;
  }

  markAsDeleted() {
    this.deleted = true;
  }

  currentRevision() {
    return {
      revision: this.revision,
      reference: this.reference,
    };
  }

  addRevision(previousDapObject) {
    this.previousRevisions = this.previousRevisions
      .concat(previousDapObject.getPreviousRevisions())
      .concat([previousDapObject.currentRevision()]);
  }

  toJSON() {
    return {
      blockchainUserId: this.blockchainUserId,
      isDeleted: this.deleted,
      type: this.type,
      object: this.object,
      revision: this.revision,
      reference: this.reference.toJSON(),
      previousRevisions: this.previousRevisionsToJSON(),
    };
  }

  /**
   *
   * @returns {{revision: number,
   *            reference: {blockHash, blockHeight, stHeaderHash, stPacketHash, objectHash}}[]}
   */
  previousRevisionsToJSON() {
    return this.previousRevisions.map(previousRevision => ({
      revision: previousRevision.revision,
      reference: previousRevision.reference.toJSON(),
    }));
  }
}

DapObject.ACTION_CREATE = 0;
DapObject.ACTION_UPDATE = 1;
DapObject.ACTION_DELETE = 2;

module.exports = DapObject;
