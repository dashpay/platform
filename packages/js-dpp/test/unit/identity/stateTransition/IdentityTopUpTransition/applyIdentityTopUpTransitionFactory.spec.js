const applyIdentityTopUpTransitionFactory = require(
  '../../../../../lib/identity/stateTransition/IdentityTopUpTransition/applyIdentityTopUpTransitionFactory',
);

const getIdentityFixture = require('../../../../../lib/test/fixtures/getIdentityFixture');
const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

describe('applyIdentityTopUpTransitionFactory', () => {
  let stateTransition;
  let applyIdentityTopUpTransition;
  let stateRepositoryMock;
  let identity;
  let fetchAssetLockTransactionOutputMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    stateTransition = getIdentityTopUpTransitionFixture();

    const output = stateTransition.getAssetLockProof().getOutput();

    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    applyIdentityTopUpTransition = applyIdentityTopUpTransitionFactory(
      stateRepositoryMock,
      fetchAssetLockTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    const balanceBeforeTopUp = identity.getBalance();

    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

    expect(identity.getBalance()).to.be.equal(balanceBeforeTopUp + balanceToTopUp);
    expect(identity.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
      );

    expect(fetchAssetLockTransactionOutputMock)
      .to.be.calledOnceWithExactly(stateTransition.getAssetLockProof());
  });
});
