const Emittery = require('emittery');
const WrongBlocksSequenceError = require('../../lib/blockchain/WrongBlocksSequenceError');

module.exports = class STHeadersIterationEventEmitter extends Emittery {
  constructor(stHeaderIterator) {
    super();

    this.stHeaderIterator = stHeaderIterator;
  }

  /**
   * Iterate over ST headers and emit events
   */
  async iterate() {
    let prevBlock;

    await this.emitSerial('begin', this.stHeaderIterator.blockIterator.getBlockHeight());

    for (; ;) {
      let done;
      let header;

      try {
        ({ done, value: header } = await this.stHeaderIterator.next());
      } catch (e) {
        if (!(e instanceof WrongBlocksSequenceError)) {
          await this.emitSerial('error', e);

          throw e;
        }

        await this.emitSerial('wrongSequence', {
          currentBlock: this.stHeaderIterator.blockIterator.getCurrentBlock(),
          previousBlock: prevBlock,
        });

        this.stHeaderIterator.reset(true);

        this.stHeaderIterator.blockIterator.setBlockHeight(prevBlock.height);

        prevBlock = null;

        // eslint-disable-next-line no-continue
        continue;
      }

      if (done) {
        await this.emitSerial('end', this.stHeaderIterator.blockIterator.getBlockHeight());

        break;
      }

      // Iterated block
      const currentBlock = this.stHeaderIterator.blockIterator.getCurrentBlock();
      if (currentBlock !== prevBlock) {
        await this.emitSerial('block', currentBlock);
        prevBlock = currentBlock;
      }

      // Iterated ST header
      await this.emitSerial('header', header);
    }
  }
};
