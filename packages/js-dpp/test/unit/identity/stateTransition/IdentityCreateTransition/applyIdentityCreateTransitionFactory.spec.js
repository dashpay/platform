const Identity = require('../../../../../lib/identity/Identity');

const applyIdentityCreateTransitionFactory = require(
  '../../../../../lib/identity/stateTransition/IdentityCreateTransition/applyIdentityCreateTransitionFactory',
);

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('../../../../../lib/version/protocolVersion');
const StateTransitionExecutionContext = require('../../../../../lib/stateTransition/StateTransitionExecutionContext');
const ReadOperation = require('../../../../../lib/stateTransition/fee/operations/ReadOperation');

describe('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let stateRepositoryMock;
  let fetchAssetLockTransactionOutputMock;
  let output;
  let executionContext;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateTransitionFixture();

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    output = stateTransition.getAssetLockProof().getOutput();

    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepositoryMock,
      fetchAssetLockTransactionOutputMock,
    );
  });

  it('should store identity created from state transition', async () => {
    executionContext.addOperation(
      new ReadOperation(1),
    );

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

    expect(stateRepositoryMock.createIdentity).to.have.been.calledOnceWithExactly(
      identity,
      executionContext,
    );

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
      executionContext,
    );

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
        executionContext,
      );

    expect(fetchAssetLockTransactionOutputMock)
      .to.be.calledOnceWithExactly(
        stateTransition.getAssetLockProof(),
        executionContext,
      );
  });
});
