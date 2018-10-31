const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');

const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

const createStateTransitionsFromBlockFactory = require('../../../lib/blockchain/createStateTransitionsFromBlockFactory');

describe('createStateTransitionsFromBlockFactory', () => {
  let rpcClientMock;
  let blocks;
  let transitions;
  let createStateTransitionsFromBlock;

  beforeEach(function beforeEach() {
    blocks = getBlockFixtures();
    transitions = getTransitionHeaderFixtures();
    rpcClientMock = new RpcClientMock(this.sinon);
    createStateTransitionsFromBlock = createStateTransitionsFromBlockFactory(rpcClientMock);
  });

  it('should create only state transitions from a block', async () => {
    const [someBlock] = blocks;
    const [transitionOne, transitionTwo] = transitions;

    // With a default constructor call it is a simple Transaction
    const nonStateTransitionTx = new StateTransitionHeader();

    // When instance of Transaction is passed to a Transaction constructor
    // it replaces itself. So we're using it to not construct a proper Transaction,
    // and not deal with `serialize` checks
    rpcClientMock.getRawTransaction
      .withArgs(nonStateTransitionTx.id)
      .resolves({
        result: nonStateTransitionTx,
      });

    someBlock.tx = [
      transitionOne.id,
      nonStateTransitionTx.id,
      transitionTwo.id,
    ];

    const stateTransitions = await createStateTransitionsFromBlock(someBlock);
    const stateTransitionHashes = stateTransitions.map(t => t.hash);

    expect(stateTransitionHashes).to.deep.equal([transitionOne.hash, transitionTwo.hash]);
  });

  it('should return state transition in a sorted order');
});
