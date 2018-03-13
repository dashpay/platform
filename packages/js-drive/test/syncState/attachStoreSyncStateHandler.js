const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);
use(sinonChai);

const Emitter = require('emittery');

const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');
const attachStoreSyncStateHandler = require('../../lib/syncState/attachStoreSyncStateHandler');

describe('attachPinSTPacketHandler', () => {
  let block;
  let state;
  let syncedBlocks;
  let syncStateRepositoryMock;
  let stHeadersReaderMock;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    syncedBlocks = getBlockFixtures();
    [block] = syncedBlocks;

    state = { };

    // Mock IPFS API
    class SyncStateRepository {
    }

    syncStateRepositoryMock = new SyncStateRepository();
    syncStateRepositoryMock.store = this.sinon.stub();

    // Mock STHeadersReader
    stHeadersReaderMock = new Emitter();
    stHeadersReaderMock.getState = () => state;
  });

  it('should store the state when next block is processed', async () => {
    attachStoreSyncStateHandler(stHeadersReaderMock, syncStateRepositoryMock);

    await stHeadersReaderMock.emitSerial('block', block);

    expect(syncStateRepositoryMock.store).to.be.calledOnce();
    expect(syncStateRepositoryMock.store).to.be.calledWith(state);
  });
});
