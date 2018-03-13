const Emittery = require('emittery');

module.exports = class ArrayBlockIterator extends Emittery {
  /**
   * @param {Object[]} blocks
   */
  constructor(blocks = []) {
    super();

    this.blocks = blocks;

    this.reset();
  }

  /**
   * Reset iterator
   */
  reset() {
    this.currentBlockIndex = 0;
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

    await this.emitSerial('block', block);

    return { done: false, value: block };
  }
};
