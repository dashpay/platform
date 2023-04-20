const {
  tendermint: {
    abci: {
      ResponseCheckTx,
    },
  },
} = require('@dashevo/abci/types');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const checkTxHandlerFactory = require('../../../../lib/abci/handlers/checkTxHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('checkTxHandlerFactory', () => {
  let checkTxHandler;
  let request;
  let stateTransitionFixture;
  let unserializeStateTransitionMock;
  let loggerMock;
  let createContextLoggerMock;

  beforeEach(function beforeEach() {
    stateTransitionFixture = getIdentityCreateTransitionFixture();

    request = {
      tx: stateTransitionFixture.toBuffer(),
    };

    unserializeStateTransitionMock = this.sinon.stub()
      .resolves(stateTransitionFixture);

    loggerMock = new LoggerMock(this.sinon);
    createContextLoggerMock = this.sinon.stub();

    checkTxHandler = checkTxHandlerFactory(
      unserializeStateTransitionMock,
      createContextLoggerMock,
      loggerMock,
    );
  });

  it('should validate a State Transition and return response', async () => {
    const response = await checkTxHandler(request);

    expect(response).to.be.an.instanceOf(ResponseCheckTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(request.tx);

    expect(createContextLoggerMock).to.be.calledOnceWithExactly(loggerMock, {
      abciMethod: 'checkTx',
    });
  });
});
