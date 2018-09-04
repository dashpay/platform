const generateDapObjectId = require('./generateDapObjectId');

class DapObject {
  /**
   * @param {string} blockchainUserId
   * @param {object} data
   * @param {Reference} reference
   */
  constructor(blockchainUserId, data, reference) {
    this.blockchainUserId = blockchainUserId;
    this.type = data.objtype;
    this.object = data;
    this.revision = data.rev;
    this.reference = reference;
  }

  getId() {
    return generateDapObjectId(this.blockchainUserId, this.object.idx);
  }

  getAction() {
    return this.object.act;
  }

  toJSON() {
    return {
      blockchainUserId: this.blockchainUserId,
      type: this.type,
      object: this.object,
      revision: this.revision,
      reference: this.reference.toJSON(),
    };
  }
}

DapObject.ACTION_CREATE = 0;
DapObject.ACTION_UPDATE = 1;
DapObject.ACTION_DELETE = 2;

module.exports = DapObject;
