const Emitter = require('emittery');

const SyncState = require('../../../../lib/sync/state/SyncState');
const STHeadersReader = require('../../../../lib/blockchain/reader/STHeadersReader');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const attachSyncHandlers = require('../../../../lib/sync/state/attachSyncHandlers');

describe('attachSyncHandlers', () => {
  let blocks;
  let readerState;
  let syncState;
  let syncStateRepositoryMock;
  let stHeadersReaderMock;

  beforeEach(function beforeEach() {
    blocks = getBlockFixtures();

    // Mock STHeadersReader
    readerState = {
      getBlocks() {
        return blocks;
      },
    };
    stHeadersReaderMock = new Emitter();
    stHeadersReaderMock.getState = () => readerState;

    // Mock SyncState
    syncState = new SyncState([], new Date());
    this.sinon.stub(syncState, 'setBlocks');
    this.sinon.stub(syncState, 'setLastSyncAt');

    // Mock SyncStateRepository
    class SyncStateRepository {
    }

    syncStateRepositoryMock = new SyncStateRepository();
    syncStateRepositoryMock.store = this.sinon.stub();

    this.sinon.useFakeTimers({ toFake: ['Date'] });
  });

  it('should store sync state when next block has processed', async () => {
    attachSyncHandlers(stHeadersReaderMock, syncState, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial(STHeadersReader.EVENTS.BLOCK, blocks[0]);

    expect(syncState.setBlocks).to.be.calledOnce();
    expect(syncState.setBlocks).to.be.calledWith(blocks);

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(syncState);
  });

  it('should store sync state when stale block has processed', async () => {
    attachSyncHandlers(stHeadersReaderMock, syncState, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial(STHeadersReader.EVENTS.STALE_BLOCK, blocks[0]);

    expect(syncState.setBlocks).to.be.calledOnce();
    expect(syncState.setBlocks).to.be.calledWith(blocks);

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(syncState);
  });

  it('should update lastSyncAt when sync has completed', async () => {
    attachSyncHandlers(stHeadersReaderMock, syncState, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial(
      STHeadersReader.EVENTS.END,
      blocks[blocks.length - 1].height,
    );

    expect(syncState.setLastSyncAt).to.be.calledOnce();
    expect(syncState.setLastSyncAt).to.be.calledWith(new Date());

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(syncState);
  });
});
