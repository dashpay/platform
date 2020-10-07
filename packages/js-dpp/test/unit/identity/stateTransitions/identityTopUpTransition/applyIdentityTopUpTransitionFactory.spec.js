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
  let getLockedTransactionOutputMock;
  let output;
  let stateRepositoryMock;
  let identity;

  beforeEach(function beforeEach() {
    output = {
      satoshis: 10000,
    };

    identity = getIdentityFixture();
    getLockedTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    stateTransition = getIdentityTopUpTransitionFixture();
    applyIdentityTopUpTransition = applyIdentityTopUpTransitionFactory(
      stateRepositoryMock,
      getLockedTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    const balanceBeforeTopUp = identity.getBalance();
    const balanceToTopUp = convertSatoshiToCredits(output.satoshis);

    await applyIdentityTopUpTransition(stateTransition);

    expect(identity.getBalance()).to.be.equal(balanceBeforeTopUp + balanceToTopUp);
    expect(identity.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

    expect(getLockedTransactionOutputMock).to.be.calledOnceWithExactly(
      stateTransition.getLockedOutPoint(),
    );
    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );
  });
});
