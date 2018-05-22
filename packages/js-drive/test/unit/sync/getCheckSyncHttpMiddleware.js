const proxyquire = require('proxyquire');

describe('getCheckSyncHttpMiddleware', () => {
  let middleware;
  let resolveSync;
  let rejectSync;
  let requestMock;
  let responseMock;
  let nextStub;
  let jaysonMock;
  let parsedRequestMock;
  let parseErrorMock;

  beforeEach(function beforeEach() {
    parsedRequestMock = { id: 1 };
    parseErrorMock = null;
    jaysonMock = {
      utils: {
        parseBody: (req, options, callback) => {
          callback(parseErrorMock, parsedRequestMock);
        },
        Request: {
          isValidRequest: this.sinon.stub().returns(true),
          isBatch: this.sinon.stub(),
        },
        response: this.sinon.stub(),
      },
      server: {
        prototype: {
          error: this.sinon.stub(),
        },
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

    requestMock = {};
    responseMock = { end: this.sinon.stub() };
    nextStub = this.sinon.stub();
  });

  it('should return middleware', () => {
    expect(middleware).to.be.a('Function');
  });

  describe('middleware', () => {
    it('should pass further if sync is complete', async function it() {
      const parseBodyStub = this.sinon.stub(jaysonMock.utils, 'parseBody');
      const parsedBody = {
        method: 'addSTPacketMethod',
        jsonrpc: '2.0',
        params: { packet: {} },
        id: 'ea9e45c4-bc90-4d79-a3ee-9c22576d69ba',
      };
      parseBodyStub.callsArgWith(2, null, parsedBody);

      const syncStateMock = {};
      resolveSync(syncStateMock);

      await new Promise((resolve) => {
        setImmediate(() => {
          middleware(requestMock, responseMock, nextStub);

          expect(requestMock.body).to.deep.equal(parsedBody);
          expect(nextStub).to.be.calledOnce();

          resolve();
        });
      });
    });
    it('should respond error if sync is not complete yet', () => {
      const errorResponse = {};
      jaysonMock.utils.response.returns(errorResponse);

      middleware(requestMock, responseMock, nextStub);

      expect(nextStub).not.to.be.called();
      expect(responseMock.end).to.be.calledOnce();
      expect(responseMock.end).to.be.calledWith(JSON.stringify(errorResponse));
    });
    it('should pass further if we can\'t parse request', () => {
      parseErrorMock = true;

      middleware(requestMock, responseMock, nextStub);

      expect(nextStub).to.be.calledOnce();
    });
    it('should pass further if request is invalid', () => {
      jaysonMock.utils.Request.isValidRequest.returns(false);

      middleware(requestMock, responseMock, nextStub);

      expect(nextStub).to.be.calledOnce();
    });
    it('should pass further if request is notification', () => {
      parsedRequestMock = {};

      middleware(requestMock, responseMock, nextStub);

      expect(nextStub).to.be.calledOnce();
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
  });
});
