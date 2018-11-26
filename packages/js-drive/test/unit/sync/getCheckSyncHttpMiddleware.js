const proxyquire = require('proxyquire');

describe('getCheckSyncHttpMiddleware', () => {
  let middleware;
  let resolveSync;
  let rejectSync;
  let responseMock;
  let nextStub;
  let jaysonMock;

  beforeEach(function beforeEach() {
    jaysonMock = {
      utils: {
        Request: {
          isValidRequest: this.sinon.stub().returns(true),
          isBatch: this.sinon.stub(),
        },
        response: this.sinon.stub(),
      },
    };

    const getCheckSyncHttpMiddleware = proxyquire('../../../lib/sync/getCheckSyncHttpMiddleware', {
      jayson: jaysonMock,
    });

    function inSyncedMock() {
      return new Promise((resolve, reject) => {
        resolveSync = resolve;
        rejectSync = reject;
      });
    }

    class RpcClient {}
    const rpcClientMock = new RpcClient();
    class SyncStateRepositoryChangeListener {}
    const changeListenerMock = new SyncStateRepositoryChangeListener();
    const checkInterval = 0.5;

    middleware = getCheckSyncHttpMiddleware(
      inSyncedMock,
      rpcClientMock,
      changeListenerMock,
      checkInterval,
    );

    responseMock = { end: this.sinon.stub() };
    nextStub = this.sinon.stub();
  });

  it('should pass further if sync is complete', async () => {
    const parsedBody = {
      method: 'addSTPacketMethod',
      jsonrpc: '2.0',
      params: { packet: {} },
      id: 'ea9e45c4-bc90-4d79-a3ee-9c22576d69ba',
    };
    const request = {
      body: parsedBody,
    };

    const syncStateMock = {};
    resolveSync(syncStateMock);

    await new Promise((resolve) => {
      setImmediate(() => {
        middleware(request, responseMock, nextStub);

        expect(nextStub).to.be.calledOnce();

        resolve();
      });
    });
  });

  it('should respond error if sync is not complete yet', () => {
    const parsedBody = {
      method: 'addSTPacketMethod',
      jsonrpc: '2.0',
      params: { packet: {} },
      id: 'ea9e45c4-bc90-4d79-a3ee-9c22576d69ba',
    };
    const request = {
      body: parsedBody,
    };
    const errorResponse = {};
    jaysonMock.utils.response.returns(errorResponse);

    middleware(request, responseMock, nextStub);

    expect(nextStub).not.to.be.called();
    expect(responseMock.end).to.be.calledOnce();
    expect(responseMock.end).to.be.calledWith(JSON.stringify(errorResponse));
  });

  it('should throw error if inSynced threw it', async () => {
    const error = new Error();
    rejectSync(error);

    await new Promise((resolve) => {
      setImmediate(() => {
        expect(middleware).to.throws(error);

        resolve();
      });
    });
  });

  it('should process batch requests');
});
