const InvalidBlockHeightError = require('./InvalidBlockHeightError');

class ArrayBlockIterator {
  /**
   * @param {Object[]} blocks
   */
  constructor(blocks = []) {
    this.blocks = blocks;

    this.reset();
  }

  /**
   * Set block height since iterator starts
   *
   * @param {number} height
   */
  setBlockHeight(height) {
    const index = this.blocks.findIndex(block => block.height === height);

    if (index === -1) {
      throw new InvalidBlockHeightError(`Block with ${height} height is not present`);
    }

    this.currentBlockIndex = index;
  }

  /**
   * Get current block height
   *
   * @return {number}
   */
  getBlockHeight() {
    const currentBlock = this.blocks[this.currentBlockIndex];

    if (currentBlock) {
      return currentBlock.height;
    }

    throw new InvalidBlockHeightError('There are no blocks');
  }

  /**
   * Reset iterator
   */
  reset() {
    this.currentBlockIndex = 0;
  }

  /**
   * @return {{next: RpcBlockIterator.next}}
   */
  [Symbol.asyncIterator]() {
    return {
      next: this.next.bind(this),
    };
  }

  /**
   * Get next block
   *
   * @return {Promise<Object>}
   */
  async next() {
    const block = this.blocks[this.currentBlockIndex];

    if (!block) {
      return { done: true };
    }

    this.currentBlockIndex++;

    return { done: false, value: block };
  }
}

module.exports = ArrayBlockIterator;
