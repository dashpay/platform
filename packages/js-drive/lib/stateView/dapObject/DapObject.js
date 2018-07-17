class DapObject {
  /**
   * @param {object} data
   * @param {Reference} reference
   */
  constructor(data, reference) {
    this.id = data.id;
    this.type = data.objtype;
    this.object = data;
    this.revision = data.rev;
    this.reference = reference;
  }

  getAction() {
    return this.object.act;
  }

  toJSON() {
    return {
      id: this.id,
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
