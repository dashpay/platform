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
  let fetchConfirmedAssetLockTransactionOutputMock;
  let output;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    output = {
      satoshis: 10000,
    };

    fetchConfirmedAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateTransitionFixture();
    applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepositoryMock,
      fetchConfirmedAssetLockTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      protocolVersion: Identity.PROTOCOL_VERSION,
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toObject()),
      balance,
      revision: 0,
    });

    expect(fetchConfirmedAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
      stateTransition.getLockedOutPoint().toString(),
    );
    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(
      identity,
    );

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId().toString(),
      publicKeyHashes,
    );
  });
});
