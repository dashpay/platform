const DAPIClient = require('../../lib/DAPIClient');

const BlockHeadersProvider = require('../../lib/BlockHeadersProvider/BlockHeadersProvider');

describe('DAPIClient - integration', () => {
  let dapiClient;
  beforeEach(() => {
    dapiClient = new DAPIClient();
  });

  describe('BlockHeadersProvider', () => {
    it('should instantiate a BlockHeadersProvider from default options', () => {
      expect(dapiClient.blockHeadersProvider).to.be.instanceOf(BlockHeadersProvider);
    });

    it('should propagate ERROR event from BlockHeadersProvider', () => {
      let emittedError;
      dapiClient.on(DAPIClient.EVENTS.ERROR, (e) => {
        emittedError = e;
      });

      const errorToEmit = new Error('test error');
      dapiClient.blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.ERROR, errorToEmit);

      expect(emittedError).to.equal(errorToEmit);
    });
  });
});
