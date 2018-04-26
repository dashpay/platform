const EventEmitter = require('events');

const isSynced = require('../../../lib/sync/isSynced');
const SyncState = require('../../../lib/sync/state/SyncState');
const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

describe('isSynced', () => {
  const checkInterval = 0.5;

  let rpcClientMock;
  let syncStateRepositoryMock;
  let changeListenerMock;

  beforeEach(function beforeEach() {
    syncStateRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    class SyncStateRepositoryChangeListener extends EventEmitter {
      // eslint-disable-next-line class-methods-use-this
      getRepository() {
        return syncStateRepositoryMock;
      }

      // eslint-disable-next-line class-methods-use-this
      listen() { }

      // eslint-disable-next-line class-methods-use-this
      stop() { }
    }

    changeListenerMock = new SyncStateRepositoryChangeListener();
    this.sinon.spy(changeListenerMock, 'listen');
    this.sinon.spy(changeListenerMock, 'stop');
    this.sinon.spy(changeListenerMock, 'removeListener');

    rpcClientMock = new RpcClientMock(this.sinon);
  });

  it('should return state if blockchain initial sync is completed and the last block in the chain is synced', async () => {
    const state = new SyncState(rpcClientMock.blocks, new Date());
    syncStateRepositoryMock.fetch.returns(state);

    rpcClientMock.mnsync.onCall(0).returns(Promise.resolve({
      result: { IsBlockchainSynced: false },
    }));
    rpcClientMock.mnsync.onCall(1).returns(Promise.resolve({
      result: { IsBlockchainSynced: true },
    }));

    const syncedState = await isSynced(rpcClientMock, changeListenerMock, checkInterval);

    expect(state).to.be.equals(syncedState);
  });

  it('should return state if last block in chain is synced', async () => {
    const state = new SyncState(rpcClientMock.blocks, new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const syncedState = await isSynced(rpcClientMock, changeListenerMock, checkInterval);

    expect(state).to.be.equals(syncedState);
  });

  it('should listen changes until last block in chain is synced', (done) => {
    const state = new SyncState([], new Date());
    syncStateRepositoryMock.fetch.returns(state);

    const isSyncedPromise = isSynced(rpcClientMock, changeListenerMock, checkInterval);

    setImmediate(() => {
      expect(changeListenerMock.listen).to.be.calledOnce();

      // State changed but sync is not completed
      state.setBlocks([rpcClientMock.blocks[0]]);
      changeListenerMock.emit('change', state);

      expect(changeListenerMock.stop).not.to.be.called();
      expect(changeListenerMock.removeListener).not.to.be.called();

      // State changed and sync is completed
      const changedState = new SyncState(rpcClientMock.blocks, new Date());
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

    const isSyncedPromise = isSynced(rpcClientMock, changeListenerMock, checkInterval);

    setImmediate(() => {
      const error = new Error();
      changeListenerMock.emit('error', error);

      expect(isSyncedPromise).to.be.rejectedWith(error);

      done();
    });
  });
});
