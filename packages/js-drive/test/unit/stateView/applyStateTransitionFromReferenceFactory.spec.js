const applyStateTransitionFromReferenceFactory = require('../../../lib/stateView/applyStateTransitionFromReferenceFactory');

const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

const getReferenceFixture = require('../../../lib/test/fixtures/getReferenceFixture');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlocksFixture');
const getTransitionFixtures = require('../../../lib/test/fixtures/getStateTransitionsFixture');

describe('applyStateTransitionFromReferenceFactory', () => {
  let rpcClientMock;
  let applyStateTransition;
  let applyStateTransitionFromReference;

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);

    applyStateTransition = this.sinon.stub();

    applyStateTransitionFromReference = applyStateTransitionFromReferenceFactory(
      applyStateTransition,
      rpcClientMock,
    );
  });

  it('should call RPC methods and applyStateTransition with proper arguments', async () => {
    const [block] = getBlockFixtures();
    const [stateTransition] = getTransitionFixtures();

    rpcClientMock.getRawTransaction
      .withArgs(stateTransition.hash)
      .resolves({
        result: stateTransition,
      });

    const reference = getReferenceFixture();

    await applyStateTransitionFromReference(reference);

    expect(rpcClientMock.getBlock).to.have.been.calledOnceWith(block.hash);
    expect(rpcClientMock.getRawTransaction).to.have.been.calledOnceWith(stateTransition.hash);
    expect(applyStateTransition).to.have.been.calledOnceWith(stateTransition, block);
  });
});
