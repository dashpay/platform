class DapObject {
  /**
   * @param {string} id
   */
  constructor(id) {
    this.id = id;
  }

  /**
   * Get DapObject JSON representation
   *
   * @returns {{id: string}}
   */
  toJSON() {
    return {
      id: this.id,
    };
  }
}

module.exports = DapObject;
