const { expect } = require('chai');
const EventEmitter = require('events');
const { Block } = require('@dashevo/dashcore-lib');
const DAPIClient = require('@dashevo/dapi-client');
const EVENTS = require('../../../../src/EVENTS');
const CONSTANTS = require('../../../../src/CONSTANTS');
const BlockHeadersSyncWorker = require('../../../../src/plugins/Workers/BlockHeadersSyncWorker/BlockHeadersSyncWorker');
const mockBlockHeadersProvider = require('../../../../src/test/mocks/mockBlockHeadersProvider');
const mockStorage = require('../../../../src/test/mocks/mockStorage');
const BlockHeadersStreamMock = require('../../../../src/test/mocks/BlockHeadersStreamMock');
const { waitOneTick } = require('../../../../src/test/utils');
const { mockHeadersChain } = require('../../../../src/test/mocks/dashcore/block');

const { BlockHeadersProvider } = DAPIClient;
const logger = require('../../../../src/logger');

describe('BlockHeadersSyncWorker - integration', () => {
  let headersChain = [];
  let blockHeadersSyncWorker;
  let historicalStreams = [];
  let continuousStream = null;

  const HEADERS_TO_KEEP = 100;
  const DEFAULT_CHAIN_HEIGHT = 500;
  const NUM_HEADERS = DEFAULT_CHAIN_HEIGHT + 1;
  const NUM_STREAMS = 5;
  const TOTAL_HEADERS_PER_STREAM = Math.ceil(NUM_HEADERS / NUM_STREAMS);

  const createWorker = async (sinon, opts = {}) => {
    const defaultOptions = {
      withAdapter: false,
    };

    const options = { ...defaultOptions, ...opts };

    const { withAdapter } = options;

    headersChain = mockHeadersChain('testnet', NUM_HEADERS);

    const worker = new BlockHeadersSyncWorker({
      maxHeadersToKeep: HEADERS_TO_KEEP,
      executeOnStart: false,
    });
    worker.logger = logger;

    historicalStreams = Array.from({ length: NUM_STREAMS }).map(() => new BlockHeadersStreamMock());
    continuousStream = new BlockHeadersStreamMock();
    const blockHeadersProvider = mockBlockHeadersProvider(
      sinon,
      historicalStreams,
      continuousStream,
      TOTAL_HEADERS_PER_STREAM,
    );
    const storage = await mockStorage({
      withAdapter,
    });

    worker.transport = {
      client: {
        blockHeadersProvider,
      },
      getBlockByHeight: (height) => Promise.resolve(new Block({
        header: headersChain[height],
        transactions: [],
      })),
    };

    worker.storage = storage;
    worker.parentEvents = new EventEmitter();

    return worker;
  };

  context('Without storage adapter', () => {
    let onStartPromise;

    before(async function before() {
      blockHeadersSyncWorker = await createWorker(this.sinon);

      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.updateChainHeight(DEFAULT_CHAIN_HEIGHT);
    });

    context('Historical sync', () => {
      // This test works with multiple headers streams and partially syncs them
      // Only first batch from the first stream that attaches to genesis block gets verified
      // The rest marked as orphaned because some batches are missing
      // and there's no connection yet between them
      it('should process first batch of from first stream and consider remaining batches as orphaned', async () => {
        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

        // Wait for the stream to start
        onStartPromise = blockHeadersSyncWorker.onStart();

        const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;
        await new Promise((resolve) => {
          blockHeadersProvider.on(BlockHeadersProvider.EVENTS.HISTORICAL_SYNC_STARTED, resolve);
        });

        // Send 1/3 of every batch
        const initialHeadersPerStream = Math.ceil(TOTAL_HEADERS_PER_STREAM / 3);
        let firstStreamBatch;
        for (let i = 0; i < NUM_STREAMS; i += 1) {
          const from = i * TOTAL_HEADERS_PER_STREAM;
          const to = from + initialHeadersPerStream;
          const headersToSend = headersChain.slice(from, to);
          if (i === 0) {
            firstStreamBatch = headersToSend;
          }
          historicalStreams[i].sendHeaders(headersToSend);
        }

        // Ensure batch of headers from the first stream was added to the storage
        // (rest are considered orphaned)
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(firstStreamBatch.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(firstStreamBatch.length - 1);

        // Ensure headers metadata
        const expectedMetaData = firstStreamBatch
          .reduce((acc, header, i) => {
            acc.set(header.hash, { height: i, time: header.time });
            return acc;
          }, new Map());
        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);
      });

      it('should process remaining batches of historical headers and handle assembled chain', async () => {
        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

        // Send data
        const prevBatchSize = Math.ceil(TOTAL_HEADERS_PER_STREAM / 3);
        const remainingHeadersPerStream = TOTAL_HEADERS_PER_STREAM - prevBatchSize;
        for (let i = 0; i < NUM_STREAMS; i += 1) {
          const from = i * TOTAL_HEADERS_PER_STREAM + prevBatchSize;
          const to = from + remainingHeadersPerStream;
          const headersToSend = headersChain
            .slice(from, to);
          historicalStreams[i].sendHeaders(headersToSend);
        }

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(DEFAULT_CHAIN_HEIGHT);

        // Ensure headers metadata
        const expectedMetaData = headersChain
          .reduce((acc, header, i) => {
            acc.set(header.hash, { height: i, time: header.time });
            return acc;
          }, new Map());

        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);

        historicalStreams.forEach((stream) => stream.end());
        await onStartPromise;
      });
    });

    context('Continuous sync', () => {
      it('should do continuous sync', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;
        await blockHeadersSyncWorker.execute();

        // New headers contains tail of the historical chain,
        // because we are syncing from the chain height
        const tail = headersChain[headersChain.length - 1];
        const newHeader = mockHeadersChain('testnet', 2, tail)[1];
        headersChain.push(newHeader);
        const newHeaders = [tail, newHeader];
        continuousStream.sendHeaders(newHeaders);

        await waitOneTick();

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        const newChainHeight = prevSyncedHeaderHeight + 1;

        // Ensure chain height update
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.chainHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.headersMetadata.get(newHeader.hash))
          .to.deep.equal({
            height: newChainHeight,
            time: newHeader.time,
          });

        await blockHeadersSyncWorker.onStop();
        expect(blockHeadersSyncWorker.syncState).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });
    });
  });

  // Set of scenarios testing behaviour with wallet storage adapter turned on
  context('With storage adapter', () => {
    before(async function before() {
      // Increase timeout because of the resource intense block headers generation
      this.timeout(3000);
      blockHeadersSyncWorker = await createWorker(this.sinon, { withAdapter: true });

      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.updateChainHeight(DEFAULT_CHAIN_HEIGHT);
    });

    // First launch partially synchronizes chain and turns off
    context('First launch', () => {
      it('should process first batches of historical headers and save to storage', async () => {
        // Wait for the stream to start
        const onStartPromise = blockHeadersSyncWorker.onStart();

        const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;
        await new Promise((resolve) => {
          blockHeadersProvider.on(BlockHeadersProvider.EVENTS.HISTORICAL_SYNC_STARTED, resolve);
        });

        const { storage } = blockHeadersSyncWorker;

        // Send 1/3 of every batch
        const initialHeadersPerStream = Math.ceil(TOTAL_HEADERS_PER_STREAM / 3);
        for (let i = 0; i < NUM_STREAMS; i += 1) {
          const from = i * TOTAL_HEADERS_PER_STREAM;
          const to = from + initialHeadersPerStream;
          const headersToSend = headersChain.slice(from, to);
          historicalStreams[i].sendHeaders(headersToSend);
        }

        // Wait for state ave
        await new Promise((resolve) => {
          storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        // Stop worker
        await blockHeadersSyncWorker.onStop();
        await onStartPromise;
        expect(blockHeadersSyncWorker.syncState)
          .to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });
    });

    // Second launch finishes historical sync and starts continuous sync
    context('Second launch', () => {
      it('should pick first batch from storage and process last batches', async () => {
        const { storage, transport: { client: { blockHeadersProvider } } } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        // Reset storage and rehydrate from adapter
        storage.reset();
        await storage.rehydrateState();

        // Reset spv chain
        blockHeadersProvider.spvChain.reset();
        blockHeadersProvider.spvChain
          .addHeaders(storage.getDefaultChainStore().state.blockHeaders);

        // Assign chain height
        chainStore.updateChainHeight(DEFAULT_CHAIN_HEIGHT);
        const onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;

        const remainingHeadersPerStream = Math
          .ceil((headersChain.length - prevSyncedHeaderHeight) / NUM_STREAMS);

        for (let i = 0; i < NUM_STREAMS; i += 1) {
          const from = (prevSyncedHeaderHeight + 1) + i * remainingHeadersPerStream;
          const to = from + remainingHeadersPerStream;
          const headersToSend = headersChain.slice(from, to);
          historicalStreams[i].sendHeaders(headersToSend);
        }

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(DEFAULT_CHAIN_HEIGHT);

        // Ensure headers metadata
        const expectedMetaData = headersChain
          .reduce((acc, header, i) => {
            acc.set(header.hash, { height: i, time: header.time });
            return acc;
          }, new Map());
        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);

        // Wait for state ave
        await new Promise((resolve) => {
          storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        historicalStreams.forEach((stream) => stream.end());
        await onStartPromise;
      });

      it('should do continuous sync and stop', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;
        await blockHeadersSyncWorker.execute();

        // New headers contains tail of the historical chain,
        // because we are syncing from the chain height
        const tail = headersChain[headersChain.length - 1];
        const newHeader = mockHeadersChain('testnet', 2, tail)[1];
        headersChain.push(newHeader);
        const newHeaders = [tail, newHeader];
        continuousStream.sendHeaders(newHeaders);

        await waitOneTick();

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        const newChainHeight = prevSyncedHeaderHeight + 1;

        // Ensure chain height update
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.chainHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.headersMetadata.get(newHeader.hash))
          .to.deep.equal({
            height: newChainHeight,
            time: newHeader.time,
          });

        // Wait for state save
        await new Promise((resolve) => {
          storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        await blockHeadersSyncWorker.onStop();
        expect(blockHeadersSyncWorker.syncState).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });
    });

    // Third launch fetches new historical headers and starts continuous sync
    context('Third launch', () => {
      it('should sync up to the new chain height', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        // Reset storage and rehydrate from adapter
        storage.reset();
        storage.lastRehydrate = null;
        await storage.rehydrateState();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight
          + CONSTANTS.STORAGE.REORG_SAFE_BLOCKS_COUNT;

        // Simulate chain update
        const headersToAdd = 50;
        const tail = headersChain[headersChain.length - 1];
        const newHeaders = mockHeadersChain('testnet', headersToAdd + 1, tail).slice(1);
        headersChain = [...headersChain, ...newHeaders];

        chainStore.updateChainHeight(prevSyncedHeaderHeight + headersToAdd);

        const onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        historicalStreams[0].sendHeaders(newHeaders);

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        // Ensure last synced header height
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(prevSyncedHeaderHeight + newHeaders.length);

        // Ensure headers metadata
        const expectedMetaData = headersChain
          .reduce((acc, header, i) => {
            acc.set(header.hash, { height: i, time: header.time });
            return acc;
          }, new Map());
        expect(chainStore.state.headersMetadata)
          .to.deep.equal(expectedMetaData);

        historicalStreams[0].end();
        await onStartPromise;
      });

      it('should do continuous sync and stop', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;
        await blockHeadersSyncWorker.execute();

        // New headers contains tail of the historical chain,
        // because we are syncing from the chain height
        const tail = headersChain[headersChain.length - 1];
        const newHeader = mockHeadersChain('testnet', 2, tail)[1];
        headersChain.push(newHeader);
        const newHeaders = [tail, newHeader];
        continuousStream.sendHeaders(newHeaders);

        await waitOneTick();

        // Ensure headers added
        const expectedHeaders = headersChain.slice(-HEADERS_TO_KEEP);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        const newChainHeight = prevSyncedHeaderHeight + 1;

        // Ensure chain height update
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.chainHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.headersMetadata.get(newHeader.hash))
          .to.deep.equal({
            height: newChainHeight,
            time: newHeader.time,
          });

        await blockHeadersSyncWorker.onStop();
        expect(blockHeadersSyncWorker.syncState).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });
    });
  });
});
