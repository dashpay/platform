const Identity = require('../../../../../lib/identity/Identity');

const applyIdentityCreateTransitionFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/applyIdentityCreateTransitionFactory',
);

const getIdentityCreateSTFixture = require('../../../../../lib/test/fixtures/getIdentityCreateSTFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

describe('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let getLockedTransactionOutputMock;
  let output;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    output = {
      satoshis: 10000,
    };

    getLockedTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateSTFixture();
    applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepositoryMock,
      getLockedTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toJSON()),
      balance,
    });

    expect(getLockedTransactionOutputMock).to.be.calledOnceWithExactly(
      stateTransition.getLockedOutPoint(),
    );
    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );
  });
});
