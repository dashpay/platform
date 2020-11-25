const applyIdentityTopUpTransitionFactory = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/applyIdentityTopUpTransitionFactory',
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

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    stateTransition = getIdentityTopUpTransitionFixture();

    applyIdentityTopUpTransition = applyIdentityTopUpTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should store identity created from state transition', async () => {
    const balanceBeforeTopUp = identity.getBalance();

    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLock().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

    expect(identity.getBalance()).to.be.equal(balanceBeforeTopUp + balanceToTopUp);
    expect(identity.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );
  });
});
