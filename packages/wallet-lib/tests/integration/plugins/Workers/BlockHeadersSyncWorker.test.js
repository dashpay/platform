const { expect } = require('chai');
const EventEmitter = require('events');
const { Block } = require('@dashevo/dashcore-lib');
const EVENTS = require('../../../../src/EVENTS');
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

  const createWorker = async (sinon, opts = {}) => {
    const defaultOptions = {
      numStreams: 1,
      numHeaders: defaultChainHeight + 1,
      withAdapter: false,
    };

    const options = { ...defaultOptions, ...opts };

    const { numStreams, numHeaders, withAdapter } = options;

    headersChain = mockHeadersChain('testnet', numHeaders);

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

  describe('With storage adapter', () => {
    describe('With 1 block headers stream', () => {
      let blockHeadersStream;
      before(async function before() {
        blockHeadersSyncWorker = await createWorker(this.sinon, { withAdapter: true });
        ([blockHeadersStream] = historicalStreams);

        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
        chainStore.chainHeight = defaultChainHeight;
      });

      it('[first launch] should process first batch of historical headers and save to storage', async () => {
        // Wait for the stream to start
        const onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        const { storage } = blockHeadersSyncWorker;

        // Send data
        const headersToSend = headersChain.slice(0, 150);
        blockHeadersStream.sendHeaders(headersToSend);

        // Wait for state ave
        await new Promise((resolve) => {
          storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        // Stop worker
        await blockHeadersSyncWorker.onStop();
        expect(blockHeadersSyncWorker.state)
          .to.equal(BlockHeadersSyncWorker.STATES.IDLE);

        await onStartPromise;
      });

      it('[second launch] should pick first batch from storage and process last batch', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        // Reset storage and rehydrate from adapter
        storage.reset();
        storage.lastRehydrate = null;
        await storage.rehydrateState();

        // Assign chain height
        chainStore.chainHeight = defaultChainHeight;
        const onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;

        // // Send data
        const headersToSend = headersChain.slice(150);
        blockHeadersStream.sendHeaders(headersToSend);
        //
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
            acc.set(header.hash, { height: i, time: header.time });
            return acc;
          }, new Map());
        expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetaData);

        // Wait for state ave
        await new Promise((resolve) => {
          storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        blockHeadersStream.end();
        await onStartPromise;
      });

      it('[second launch] should do continuous sync and stop', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();
        const walletStore = storage.getDefaultWalletStore();

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

        //
        // Ensure headers added
        const expectedHeaders = headersChain.slice(-headersToKeep);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        const newChainHeight = prevSyncedHeaderHeight + 1;

        // Ensure chain height update
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.blockHeight)
          .to.equal(newChainHeight);
        expect(walletStore.state.lastKnownBlock)
          .to.deep.equal({
            height: newChainHeight,
          });
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
        expect(blockHeadersSyncWorker.state).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });

      it('[third launch] should sync up to the new chain height', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        // Reset storage and rehydrate from adapter
        storage.reset();
        storage.lastRehydrate = null;
        await storage.rehydrateState();

        const prevSyncedHeaderHeight = chainStore.state.lastSyncedHeaderHeight;

        // Simulate chain update
        const headersToAdd = 50;
        const tail = headersChain[headersChain.length - 1];
        const newHeaders = mockHeadersChain('testnet', headersToAdd + 1, tail)
          .slice(1);
        headersChain = [...headersChain, ...newHeaders];

        chainStore.chainHeight = prevSyncedHeaderHeight + headersToAdd;

        const onStartPromise = blockHeadersSyncWorker.onStart();
        await waitOneTick();

        historicalStreams[0].sendHeaders(newHeaders);

        //
        // Ensure headers added
        const expectedHeaders = headersChain.slice(-headersToKeep);
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

      it('[third launch] should do continuous sync and stop', async () => {
        const { storage } = blockHeadersSyncWorker;
        const chainStore = storage.getDefaultChainStore();
        const walletStore = storage.getDefaultWalletStore();

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

        //
        // Ensure headers added
        const expectedHeaders = headersChain.slice(-headersToKeep);
        expect(chainStore.state.blockHeaders.map((header) => header.toString()))
          .to.deep.equal(expectedHeaders.map((header) => header.toString()));

        const newChainHeight = prevSyncedHeaderHeight + 1;

        // Ensure chain height update
        expect(chainStore.state.lastSyncedHeaderHeight)
          .to.equal(newChainHeight);
        expect(chainStore.state.blockHeight)
          .to.equal(newChainHeight);
        expect(walletStore.state.lastKnownBlock)
          .to.deep.equal({
            height: newChainHeight,
          });
        expect(chainStore.state.headersMetadata.get(newHeader.hash))
          .to.deep.equal({
            height: newChainHeight,
            time: newHeader.time,
          });

        await blockHeadersSyncWorker.onStop();
        expect(blockHeadersSyncWorker.state).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      });
    });
  });

  describe('Without storage adapter', () => {
    describe('With 1 block headers stream', () => {
      it('should process first batch of historical headers');
      it('should process last batch of historical headers');
      it('should do continuous sync');
      it('should sync from the beginning after the restart');
    });
  });

  describe.skip('#onStart', () => {
    describe('Historical sync with 1 stream', () => {
      let onStartPromise;
      let blockHeadersStream;
      before(async function before() {
        blockHeadersSyncWorker = await createWorker(this.sinon);
        ([blockHeadersStream] = historicalStreams);

        const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
        chainStore.chainHeight = defaultChainHeight;
      });

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
