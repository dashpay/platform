const DAPIClient = require('@dashevo/dapi-client/lib');
const { genesis } = require('@dashevo/dash-spv');

const BlockHeadersSyncWorker = require('../../../../src/plugins/Workers/BlockHeadersSyncWorker/BlockHeadersSyncWorker');
const mockBlockHeadersProvider = require('../../../../src/test/mocks/mockBlockHeadersProvider');
const mockStorage = require('../../../../src/test/mocks/mockStorage');
const BlockHeadersStreamMock = require('../../../../src/test/mocks/BlockHeadersStreamMock');
const { waitOneTick } = require('../../../../src/test/utils');

const { BlockHeadersProvider } = DAPIClient;

describe('BlockHeadersSyncWorker', () => {
  let blockHeadersSyncWorker;
  let historicalStreams = [];
  let continuousStream = null;

  const headersToKeep = 10;
  const defaultChainHeight = 1000;

  const createWorker = (sinon, numStreams = 1) => {
    const worker = new BlockHeadersSyncWorker({
      headersToKeep,
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
    describe('With 1 stream', () => {
      beforeEach(function beforeEach() {
        blockHeadersSyncWorker = createWorker(this.sinon);

        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
        chainStore.chainHeight = defaultChainHeight;
      });

      it('should kickstart historical sync', async () => {
        const promise = blockHeadersSyncWorker.onStart();

        await waitOneTick();

        // const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

        // blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
        // historicalStreams.forEach(str).end();
        historicalStreams[0].end();

        await promise;
        // await promi;
        // await blockHeadersSyncWorker.start();
        // await blockHeadersSyncWorker.stop();
      });
    });
  });
});
