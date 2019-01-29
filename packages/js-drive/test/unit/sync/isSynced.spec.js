const EventEmitter = require('events');

const isSynced = require('../../../lib/sync/isSynced');
const SyncState = require('../../../lib/sync/state/SyncState');
const SyncInfo = require('../../../lib/sync/info/SyncInfo');
const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

describe('isSynced', () => {
  const checkInterval = 0.5;

  let rpcClientMock;
  let syncStateRepositoryMock;
  let changeListenerMock;
  let getSyncInfo;

  beforeEach(function beforeEach() {
    syncStateRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    class SyncStateRepositoryChangeListener extends EventEmitter {
      getRepository() {
        return syncStateRepositoryMock;
      }

      listen() { }

      stop() { }
    }

    changeListenerMock = new SyncStateRepositoryChangeListener();
    this.sinon.spy(changeListenerMock, 'listen');
    this.sinon.spy(changeListenerMock, 'stop');
    this.sinon.spy(changeListenerMock, 'removeListener');

    rpcClientMock = new RpcClientMock(this.sinon);

    getSyncInfo = this.sinon.stub();
  });

  it('should return state if blockchain initial sync is completed and the last block in the chain is synced', async () => {
    const state = new SyncState(rpcClientMock.blocks, new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, new Date(), null, null, blockHash, true);

    getSyncInfo.onCall(0).resolves(
      new SyncInfo(null, null, null, null, null, null, false),
    );
    getSyncInfo.onCall(1).resolves(info);

    const syncInfo = await isSynced(getSyncInfo, changeListenerMock, checkInterval);

    expect(info).to.be.equals(syncInfo);
  });

  it('should return state if last block in chain is synced', async () => {
    const state = new SyncState(rpcClientMock.blocks, new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, new Date(), null, null, blockHash, true);
    getSyncInfo.resolves(info);

    const syncInfo = await isSynced(getSyncInfo, changeListenerMock, checkInterval);

    expect(info).to.be.equals(syncInfo);
  });

  it('should listen changes until last block in chain is synced', (done) => {
    const state = new SyncState([], new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    let info = new SyncInfo(null, blockHash, new Date(), null, null, null, true);
    getSyncInfo.resolves(info);
    const isSyncedPromise = isSynced(getSyncInfo, changeListenerMock, checkInterval);

    setImmediate(() => {
      expect(changeListenerMock.listen).to.be.calledOnce();

      // State changed but sync is not completed
      state.setBlocks([rpcClientMock.blocks[0]]);
      changeListenerMock.emit('change', state);

      expect(changeListenerMock.stop).not.to.be.called();
      expect(changeListenerMock.removeListener).not.to.be.called();

      // State changed and sync is completed
      const changeTime = new Date();
      changeTime.setSeconds(changeTime.getSeconds() + 1);

      info = new SyncInfo(null, blockHash, changeTime, null, null, null, true);
      getSyncInfo.resolves(info);

      const changedState = new SyncState(rpcClientMock.blocks, changeTime);
      changeListenerMock.emit('change', changedState);

      expect(changeListenerMock.removeListener).to.be.calledOnce();
      expect(changeListenerMock.removeListener).to.be.calledWith('change');

      expect(changeListenerMock.stop).to.be.calledOnce();

      expect(isSyncedPromise).become(changedState);

      done();
    });
  });

  it('should return error if change listener emits error', (done) => {
    const state = new SyncState([], new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, new Date(), null, null, null, true);
    getSyncInfo.resolves(info);

    const isSyncedPromise = isSynced(getSyncInfo, changeListenerMock, checkInterval);

    setImmediate(() => {
      const error = new Error();
      changeListenerMock.emit('error', error);

      expect(isSyncedPromise).to.be.rejectedWith(error);

      done();
    });
  });

  it('should not remove change listener if SyncState and UpdateState have empty lastSyncAt', (done) => {
    const state = new SyncState([], null);
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, new Date(), null, null, null, true);
    getSyncInfo.resolves(info);

    const isSyncedPromise = isSynced(getSyncInfo, changeListenerMock, checkInterval);

    setImmediate(() => {
      const changedState = new SyncState(rpcClientMock.blocks, null);
      changeListenerMock.emit('change', changedState);

      expect(changeListenerMock.removeListener).to.be.not.calledOnce();
      expect(changeListenerMock.removeListener).to.be.not.calledWith('change');

      expect(changeListenerMock.stop).to.be.not.calledOnce();

      expect(isSyncedPromise).become(changedState);

      done();
    });
  });

  it('should not remove change listener if SyncInfo has empty lastSyncAt', (done) => {
    const state = new SyncState([], null);
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, null, null, null, null, true);
    getSyncInfo.resolves(info);

    const isSyncedPromise = isSynced(getSyncInfo, changeListenerMock, checkInterval);

    setImmediate(() => {
      const changedState = new SyncState(rpcClientMock.blocks, new Date());
      changeListenerMock.emit('change', changedState);

      expect(changeListenerMock.removeListener).to.be.calledOnce();
      expect(changeListenerMock.removeListener).to.be.calledWith('change');

      expect(changeListenerMock.stop).to.be.calledOnce();

      expect(isSyncedPromise).become(changedState);

      done();
    });
  });

  it('should not remove change listener if UpdateSyncState has empty lastSyncAt', (done) => {
    const state = new SyncState([], new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const blockHash = 'somehash';
    const info = new SyncInfo(null, blockHash, new Date(), null, null, null, true);
    getSyncInfo.resolves(info);

    const isSyncedPromise = isSynced(getSyncInfo, changeListenerMock, checkInterval);

    setImmediate(() => {
      const changedState = new SyncState(rpcClientMock.blocks, null);
      changeListenerMock.emit('change', changedState);

      expect(changeListenerMock.removeListener).to.be.not.calledOnce();
      expect(changeListenerMock.removeListener).to.be.not.calledWith('change');

      expect(changeListenerMock.stop).to.be.not.calledOnce();

      expect(isSyncedPromise).become(changedState);

      done();
    });
  });
});
