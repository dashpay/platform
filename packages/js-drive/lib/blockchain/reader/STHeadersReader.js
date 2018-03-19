const Emittery = require('emittery');

class STHeadersReader extends Emittery {
  /**
   * @param {StateTransitionHeaderIterator} stHeaderIterator
   * @param {STHeadersReaderState} state
   */
  constructor(stHeaderIterator, state) {
    super();

    this.stHeaderIterator = stHeaderIterator;
    this.state = state;

    const { blockIterator } = this.stHeaderIterator;

    this.isHeightChangedInBlockHandler = false;
    this.initialBlockHeight = blockIterator.getBlockHeight();

    blockIterator.on('block', this.onBlockHandler.bind(this));

    const lastBlock = this.state.getLastBlock();

    if (lastBlock) {
      blockIterator.setBlockHeight(lastBlock.height + 1);
    }
  }

  /**
   * Read ST headers and emit events
   */
  async read() {
    await this.emitSerial('begin', this.stHeaderIterator.blockIterator.getBlockHeight());

    for (; ;) {
      const { done, value: header } = await this.stHeaderIterator.next();

      // when we get next block 'block' event is fired and we check
      // sequence with previous block in 'onBlockHandler'
      // if sequence is wrong we change current height to previous
      // and start iteration again.
      if (this.isHeightChangedInBlockHandler) {
        this.isHeightChangedInBlockHandler = false;

        // eslint-disable-next-line no-continue
        continue;
      }

      if (done) {
        await this.emitSerial('end', this.stHeaderIterator.blockIterator.getBlockHeight());

        break;
      }

      // Iterated ST header
      await this.emitSerial('header', header);
    }
  }

  /**
   * Get state
   *
   * @return {STHeadersReaderState}
   */
  getState() {
    return this.state;
  }

  /**
   * @private
   * @return {Promise<void>}
   */
  async onBlockHandler(currentBlock) {
    const previousBlock = this.state.getLastBlock();

    if (this.isNotAbleToVerifySequence(currentBlock, previousBlock)) {
      return this.resetIterator(this.initialBlockHeight, currentBlock);
    }

    if (this.isWrongSequence(currentBlock, previousBlock)) {
      return this.resetIterator(previousBlock.height, currentBlock);
    }

    this.state.addBlock(currentBlock);

    return this.emitSerial('block', currentBlock);
  }

  /**
   * @private
   * @param currentBlock
   * @param previousBlock
   * @return {boolean}
   */
  isNotAbleToVerifySequence(currentBlock, previousBlock) {
    return !previousBlock &&
      currentBlock.height !== this.initialBlockHeight;
  }

  /**
   * @private
   * @param currentBlock
   * @param previousBlock
   * @return {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isWrongSequence(currentBlock, previousBlock) {
    return previousBlock &&
      currentBlock.previousblockhash &&
      currentBlock.previousblockhash !== previousBlock.hash;
  }

  /**
   * @private
   * @param {number} height
   * @param {Object} currentBlock
   * @return {Promise<void>}
   */
  async resetIterator(height, currentBlock) {
    await this.emitSerial('wrongSequence', currentBlock);

    this.stHeaderIterator.reset(true);
    this.stHeaderIterator.blockIterator.setBlockHeight(height);

    this.isHeightChangedInBlockHandler = true;

    if (height === this.initialBlockHeight) {
      this.state.clear();
    } else {
      this.state.removeLastBlock();
    }
  }
}

module.exports = STHeadersReader;
