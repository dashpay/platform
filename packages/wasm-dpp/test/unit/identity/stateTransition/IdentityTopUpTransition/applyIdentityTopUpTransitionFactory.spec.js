const applyIdentityTopUpTransitionFactory = require(
  '@dashevo/dpp/lib/identity/stateTransition/IdentityTopUpTransition/applyIdentityTopUpTransitionFactory',
);

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');

const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

describe('applyIdentityTopUpTransitionFactory', () => {
  let stateTransition;
  let applyIdentityTopUpTransition;
  let stateRepositoryMock;
  let identity;
  let fetchAssetLockTransactionOutputMock;
  let executionContext;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    stateTransition = getIdentityTopUpTransitionFixture();

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

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

    expect(stateRepositoryMock.updateIdentity).to.have.been.calledOnceWithExactly(
      identity,
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

  it('should add topup amount to identity balance on dry run', async () => {
    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    executionContext.enableDryRun();

    await applyIdentityTopUpTransition(stateTransition);

    executionContext.disableDryRun();

    expect(stateRepositoryMock.addToIdentityBalance).to.have.been.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      balanceToTopUp,
      executionContext,
    );

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
        executionContext,
      );

    expect(fetchAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
      stateTransition.getAssetLockProof(),
      executionContext,
    );
  });
});
