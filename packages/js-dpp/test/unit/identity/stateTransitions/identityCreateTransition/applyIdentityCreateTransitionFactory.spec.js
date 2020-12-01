const Identity = require('../../../../../lib/identity/Identity');

const applyIdentityCreateTransitionFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/applyIdentityCreateTransitionFactory',
);

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

describe('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateTransitionFixture();

    applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should store identity created from state transition', async () => {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(stateTransition.getAssetLock().getOutput().satoshis);

    const identity = new Identity({
      protocolVersion: Identity.PROTOCOL_VERSION,
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toObject()),
      balance,
      revision: 0,
    });

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(stateRepositoryMock.storeAssetLockTransactionOutPoint).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLock().getOutPoint(),
      );
  });
});
