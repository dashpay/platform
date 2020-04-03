const {
  abci: {
    ResponseCheckTx,
  },
} = require('abci/types');

const getIdentityCreateSTFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateSTFixture');

const checkTxHandlerFactory = require('../../../../lib/abci/handlers/checkTxHandlerFactory');

describe('checkTxHandlerFactory', () => {
  let checkTxHandler;
  let request;
  let stateTransitionFixture;
  let unserializeStateTransitionMock;

  beforeEach(function beforeEach() {
    stateTransitionFixture = getIdentityCreateSTFixture();

    request = {
      tx: stateTransitionFixture.serialize(),
    };

    unserializeStateTransitionMock = this.sinon.stub()
      .resolves(stateTransitionFixture);

    checkTxHandler = checkTxHandlerFactory(
      unserializeStateTransitionMock,
    );
  });

  it('should validate a State Transition and return response with code 0', async () => {
    const response = await checkTxHandler(request);

    expect(response).to.be.an.instanceOf(ResponseCheckTx);
    expect(response.code).to.equal(0);

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(request.tx);
  });
});
