const Identity = require('../../../../../lib/identity/Identity');

const applyIdentityCreateTransitionFactory = require(
  '../../../../../lib/identity/stateTransition/IdentityCreateTransition/applyIdentityCreateTransitionFactory',
);

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('../../../../../lib/version/protocolVersion');

describe('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let stateRepositoryMock;
  let fetchAssetLockTransactionOutputMock;
  let output;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateTransitionFixture();

    output = stateTransition.getAssetLockProof().getOutput();

    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepositoryMock,
      fetchAssetLockTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(
      output.satoshis,
    );

    const identity = new Identity({
      protocolVersion: protocolVersion.latestVersion,
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

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
      );

    expect(fetchAssetLockTransactionOutputMock)
      .to.be.calledOnceWithExactly(stateTransition.getAssetLockProof());
  });
});
