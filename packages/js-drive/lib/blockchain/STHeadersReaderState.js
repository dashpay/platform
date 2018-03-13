module.exports = class STHeadersReaderState {
  /**
   * @param {Array} blocks
   * @param {number} blocksLimit
   */
  constructor(blocks = [], blocksLimit = 12) {
    this.blocksLimit = blocksLimit;

    this.setBlocks(blocks);
  }

  /**
   * Set blocks
   *
   * @param {Object[]} blocks
   */
  setBlocks(blocks) {
    this.clear();

    blocks.forEach(this.addBlock.bind(this));
  }

  /**
   * Add block
   *
   * @param {Object} block
   */
  addBlock(block) {
    if (this.currentHeight && block.height !== this.currentHeight + 1) {
      throw new Error('Wrong block sequence');
    }

    this.currentHeight = block.height;
    this.blocksCount++;

    if (!this.firstBlockHeight) {
      this.firstBlockHeight = this.currentHeight;
    }

    this.blocks[this.currentHeight] = block;

    this.trimToLimit();
  }

  /**
   * Get last block
   *
   * @return {Object}
   */
  getLastBlock() {
    return this.blocks[this.currentHeight];
  }

  /**
   * Remove last block
   */
  removeLastBlock() {
    if (!this.blocks[this.currentHeight]) {
      return;
    }

    delete this.blocks[this.currentHeight];
    this.currentHeight--;
    this.blocksCount--;

    if (this.blocksCount === 0) {
      this.clear();
    }
  }

  /**
   * Get blocks
   *
   * @return {Object[]}
   */
  getBlocks() {
    return this.blocks.slice(this.firstBlockHeight);
  }

  /**
   * Clear state
   */
  clear() {
    this.blocks = [];

    this.blocksCount = 0;

    this.currentHeight = 0;
    this.firstBlockHeight = 0;
  }

  /**
   * @private
   */
  trimToLimit() {
    if (this.blocksCount <= this.blocksLimit) {
      return;
    }

    delete this.blocks[this.firstBlockHeight];
    this.firstBlockHeight++;
    this.blocksCount--;
  }
};
