class AbstractRootTreeLeaf {
  /**
   * @param {number} index
   */
  constructor(index) {
    this.index = index;
  }

  /**
   * Get leaf index
   *
   * @return {number}
   */
  getIndex() {
    return this.index;
  }

  /**
   * @abstract
   */
  getHash() {
    throw new Error('Is not implemented');
  }
}

module.exports = AbstractRootTreeLeaf;
