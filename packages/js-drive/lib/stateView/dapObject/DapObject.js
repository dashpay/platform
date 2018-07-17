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

  isNew() {
    return this.object && this.object.act === 0;
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

module.exports = DapObject;
