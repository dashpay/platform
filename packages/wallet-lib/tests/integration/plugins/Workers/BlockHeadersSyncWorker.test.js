const { expect } = require('chai');

const BlockHeadersSyncWorker = require('../../../../src/plugins/Workers/BlockHeadersSyncWorker/BlockHeadersSyncWorker');
const mockBlockHeadersProvider = require('../../../../src/test/mocks/mockBlockHeadersProvider');
const mockStorage = require('../../../../src/test/mocks/mockStorage');
const BlockHeadersStreamMock = require('../../../../src/test/mocks/BlockHeadersStreamMock');
const { waitOneTick } = require('../../../../src/test/utils');
const mockHeadersChain = require('../../../../src/test/mocks/mockHeadersChain');

describe('BlockHeadersSyncWorker', () => {
  let headersChain = [];
  let blockHeadersSyncWorker;
  let historicalStreams = [];
  let continuousStream = null;

  const headersToKeep = 100;
  const defaultChainHeight = 500;

  const createWorker = (sinon, numStreams = 1) => {
    headersChain = mockHeadersChain('testnet', defaultChainHeight);

    const worker = new BlockHeadersSyncWorker({
      maxHeadersToKeep: headersToKeep,
      executeOnStart: false,
    });

    historicalStreams = Array.from({ length: numStreams }).map(() => new BlockHeadersStreamMock());
    continuousStream = new BlockHeadersStreamMock();
    const blockHeadersProvider = mockBlockHeadersProvider(
      sinon,
      historicalStreams,
      continuousStream,
    );
    const storage = mockStorage();

    worker.transport = {
      client: {
        blockHeadersProvider,
      },
    };

    worker.storage = storage;

    return worker;
  };

  describe('#onStart', () => {
    let blockHeadersStream;
    before(function before() {
      blockHeadersSyncWorker = createWorker(this.sinon);
      ([blockHeadersStream] = historicalStreams);

      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.chainHeight = defaultChainHeight;
    });

    describe('First sync with 1 stream', () => {
      let onStartPromise;

      it('should process first batch', async () => {
        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

        // Wait for the stream to start
        onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        // Send data
        const headersToSend = headersChain.slice(0, 150);
        blockHeadersStream.sendHeaders(headersToSend);

        // Ensure headers added
        const expectedHeaders = headersToSend.slice(-headersToKeep);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(headersToSend.length - 1);

        // Ensure headers metadata
        const expectedMetaData = headersToSend
          .reduce((acc, header, i) => {
            Object.assign(acc, { [header.hash]: { height: i, time: header.time } });
            return acc;
          }, {});
        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);
      });

      it('should process last batch', async () => {
        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;

        // Send data
        const headersToSend = headersChain.slice(150);
        blockHeadersStream.sendHeaders(headersToSend);

        // Ensure headers added
        const expectedHeaders = headersToSend.slice(-headersToKeep);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(prevSyncedHeaderHeight + headersToSend.length);

        // Ensure headers metadata
        const expectedMetaData = headersChain
          .reduce((acc, header, i) => {
            Object.assign(acc, { [header.hash]: { height: i, time: header.time } });
            return acc;
          }, {});
        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);

        blockHeadersStream.end();
        await onStartPromise;
      });
    });
  });
});
