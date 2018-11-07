const applyStateTransitionFromReferenceFactory = require('../../../lib/stateView/applyStateTransitionFromReferenceFactory');

const Reference = require('../../../lib/stateView/Reference');

const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const getTransitionFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('applyStateTransitionFromReference', () => {
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
    const [transition] = getTransitionFixtures();

    rpcClientMock.getRawTransaction
      .withArgs(transition.hash)
      .resolves({
        result: transition,
      });

    const reference = new Reference(
      block.hash,
      block.height,
      transition.hash,
      null,
      null,
    );

    await applyStateTransitionFromReference(reference);

    expect(rpcClientMock.getBlock).to.be.calledOnce();
    expect(rpcClientMock.getBlock).to.be.calledWith(block.hash);

    expect(rpcClientMock.getRawTransaction).to.be.calledOnce();
    expect(rpcClientMock.getRawTransaction).to.be.calledWith(transition.hash);

    expect(applyStateTransition).to.be.calledOnce();
    expect(applyStateTransition).to.be.calledWith(transition, block);
  });
});
