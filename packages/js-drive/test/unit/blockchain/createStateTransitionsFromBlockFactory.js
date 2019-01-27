const getBlockFixtures = require('../../../lib/test/fixtures/getBlocksFixture');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getStateTransitionsFixture');

const StateTransitionHeader = require('../../../lib/blockchain/StateTransitionHeader');

const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

const createStateTransitionsFromBlockFactory = require('../../../lib/blockchain/createStateTransitionsFromBlockFactory');

const shuffleArray = require('../../../lib/util/shuffleArray');

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
      .withArgs(nonStateTransitionTx.hash)
      .resolves({
        result: nonStateTransitionTx,
      });

    someBlock.tx = [
      transitionOne.hash,
      nonStateTransitionTx.hash,
      transitionTwo.hash,
    ];

    const stateTransitions = await createStateTransitionsFromBlock(someBlock);
    const stateTransitionHashes = stateTransitions.map(t => t.hash);

    expect(stateTransitionHashes).to.include(transitionOne.hash);
    expect(stateTransitionHashes).to.include(transitionTwo.hash);
    expect(stateTransitionHashes).to.not.include(nonStateTransitionTx.hash);
  });

  it('should return state transition in a sorted order', async () => {
    const [someBlock] = blocks;

    const groupOneRegTxId = 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b';
    const groupTwoRegTxId = 'e1cc3672035d3db961885a0511a87d798cbcbcfa73392d7aa822b0bed6a285b1';

    const createTransitionGroup = (regTxId, transitionArray) => {
      const result = [];

      for (const transition of transitionArray) {
        const lastAddedTransition = result[result.length - 1];

        transition.extraPayload.regTxId = regTxId;

        if (lastAddedTransition) {
          transition.extraPayload.hashPrevSubTx = lastAddedTransition.hash;
        }

        rpcClientMock.getRawTransaction
          .withArgs(transition.hash)
          .resolves({
            result: transition.serialize(),
          });

        result.push(new StateTransitionHeader(transition));
      }

      return result;
    };

    const groupOne = createTransitionGroup(groupOneRegTxId, transitions);
    const groupTwo = createTransitionGroup(groupTwoRegTxId, transitions);

    const transitionArray = groupOne.concat(groupTwo);

    shuffleArray(transitionArray);

    someBlock.tx = transitionArray.map(t => t.hash);

    const stateTransitions = await createStateTransitionsFromBlock(someBlock);

    for (let i = 1; i < groupOne.length; i++) {
      expect(stateTransitions[i].extraPayload.hashPrevSubTx).to.be
        .equal(stateTransitions[i - 1].hash);
    }

    for (let i = groupOne.length + 1; i < groupOne.length + groupTwo.length; i++) {
      expect(stateTransitions[i].extraPayload.hashPrevSubTx).to.be
        .equal(stateTransitions[i - 1].hash);
    }
  });
});
