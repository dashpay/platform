const Emittery = require('emittery');

const ResetIteratorError = require('./ResetIteratorError');

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
    await this.emitSerial(
      STHeadersReader.EVENTS.BEGIN,
      this.stHeaderIterator.blockIterator.getBlockHeight(),
    );

    for (; ;) {
      let done;
      let header;

      try {
        ({ done, value: header } = await this.stHeaderIterator.next());
      } catch (e) {
        if (e instanceof ResetIteratorError) {
          // eslint-disable-next-line no-continue
          continue;
        }

        throw e;
      }

      if (done) {
        await this.emitSerial(
          STHeadersReader.EVENTS.END,
          this.stHeaderIterator.blockIterator.getBlockHeight(),
        );

        break;
      }

      // Iterated ST header
      await this.emitSerial(STHeadersReader.EVENTS.HEADER, {
        header,
        block: this.stHeaderIterator.blockIterator.currentBlock,
      });
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
      this.state.clear();

      await this.emit(STHeadersReader.EVENTS.RESET);

      return this.restartIterator(this.initialBlockHeight);
    }

    if (this.isWrongSequence(currentBlock, previousBlock)) {
      this.state.removeLastBlock();

      await this.emitSerial(STHeadersReader.EVENTS.STALE_BLOCK, previousBlock);

      if (previousBlock.height >= currentBlock.height) {
        return this.onBlockHandler(currentBlock);
      }

      return this.restartIterator(currentBlock.height - 1);
    }

    this.state.addBlock(currentBlock);

    return this.emitSerial(STHeadersReader.EVENTS.BLOCK, currentBlock);
  }

  /**
   * @private
   * @param currentBlock
   * @param previousBlock
   * @return {boolean}
   */
  isNotAbleToVerifySequence(currentBlock, previousBlock) {
    if (!previousBlock) {
      if (currentBlock.height !== this.initialBlockHeight) {
        // The state doesn't contain synced blocks and
        // current block's height is not initial blocks height
        return true;
      }
    } else if (currentBlock.height < previousBlock.height &&
      previousBlock.height - currentBlock.height - 2 > this.state.getBlocksLimit()) {
      // The state doesn't contain previous block for current block
      return true;
    }

    return false;
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
   * @throws ResetIteratorError
   * @return {Promise<void>}
   */
  async restartIterator(height) {
    this.stHeaderIterator.reset(true);
    this.stHeaderIterator.blockIterator.setBlockHeight(height);

    throw new ResetIteratorError();
  }
}

STHeadersReader.EVENTS = {
  BEGIN: 'begin',
  HEADER: 'header',
  BLOCK: 'block',
  RESET: 'reset',
  STALE_BLOCK: 'staleBlock',
  END: 'end',
};

module.exports = STHeadersReader;
