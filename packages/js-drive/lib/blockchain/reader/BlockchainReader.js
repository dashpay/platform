const ReaderMediator = require('./BlockchainReaderMediator');
const RestartBlockchainReaderError = require('./errors/RestartBlockchainReaderError');
const IgnoreStateTransitionError = require('./errors/IgnoreStateTransitionError');

class BlockchainReader {
  /**
   * @param {RpcBlockIterator|ArrayBlockIterator} blockIterator
   * @param {BlockchainReaderMediator} readerMediator
   * @param {createStateTransitionsFromBlock} createStateTransitionsFromBlock
   */
  constructor(blockIterator, readerMediator, createStateTransitionsFromBlock) {
    this.blockIterator = blockIterator;
    this.readerMediator = readerMediator;
    this.createStateTransitionsFromBlock = createStateTransitionsFromBlock;
  }

  /**
   * Read ST headers and emit events
   *
   * @return {Promise<number>}
   */
  async read(height) {
    this.blockIterator.setBlockHeight(height);

    let block;
    for await (block of this.blockIterator) {
      let stateTransition = null;

      try {
        await this.readerMediator.emitSerial(ReaderMediator.EVENTS.BLOCK_BEGIN, block);

        for (stateTransition of await this.createStateTransitionsFromBlock(block)) {
          try {
            await this.readerMediator.emitSerial(ReaderMediator.EVENTS.STATE_TRANSITION, {
              stateTransition,
              block,
            });
          } catch (error) {
            let lastError = error;

            try {
              await this.readerMediator.emitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ERROR, {
                error,
                block,
                stateTransition,
              });
            } catch (e) {
              if (e instanceof IgnoreStateTransitionError) {
                continue;
              }

              lastError = e;
            }

            throw lastError;
          }
        }

        this.readerMediator.getState().addBlock(block);

        await this.readerMediator.emitSerial(ReaderMediator.EVENTS.BLOCK_END, block);
      } catch (error) {
        let lastError = error;

        try {
          await this.readerMediator.emitSerial(ReaderMediator.EVENTS.BLOCK_ERROR, {
            error,
            block,
            stateTransition,
          });
        } catch (e) {
          if (e instanceof RestartBlockchainReaderError) {
            return this.read(e.getHeight());
          }

          lastError = e;
        }

        throw lastError;
      }
    }

    return block ? block.height : 0;
  }
}


module.exports = BlockchainReader;
