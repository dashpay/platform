const Emitter = require('emittery');

const SyncState = require('../../../lib/sync/state/SyncState');

const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const attachStoreSyncStateHandler = require('../../../lib/sync/state/attachStoreSyncStateHandler');

describe('attachPinSTPacketHandler', () => {
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
    class SyncStateRepository { }
    syncStateRepositoryMock = new SyncStateRepository();
    syncStateRepositoryMock.store = this.sinon.stub();

    this.sinon.useFakeTimers({ toFake: ['Date'] });
  });

  it('should store sync state when next block has processed', async () => {
    attachStoreSyncStateHandler(stHeadersReaderMock, syncState, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial('block', blocks[0]);

    expect(syncState.setBlocks).to.be.calledOnce();
    expect(syncState.setBlocks).to.be.calledWith(blocks);

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(syncState);
  });

  it('should update lastSyncAt when sync has completed', async () => {
    attachStoreSyncStateHandler(stHeadersReaderMock, syncState, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial('end', blocks[blocks.length - 1].height);

    expect(syncState.setLastSyncAt).to.be.calledOnce();
    expect(syncState.setLastSyncAt).to.be.calledWith(new Date());

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(syncState);
  });
});
