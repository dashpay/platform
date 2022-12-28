const applyIdentityTopUpTransitionFactory = require(
  '../../../../../lib/identity/stateTransition/IdentityTopUpTransition/applyIdentityTopUpTransitionFactory',
);

const getIdentityFixture = require('../../../../../lib/test/fixtures/getIdentityFixture');
const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const { convertSatoshiToCredits } = require('../../../../../lib/identity/creditsConverter');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const StateTransitionExecutionContext = require('../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('applyIdentityTopUpTransitionFactory', () => {
  let stateTransition;
  let applyIdentityTopUpTransition;
  let stateRepositoryMock;
  let fetchAssetLockTransactionOutputMock;
  let executionContext;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

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

  it('should add topup amount to identity balance', async () => {
    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

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

    expect(fetchAssetLockTransactionOutputMock)
      .to.be.calledOnceWithExactly(
        stateTransition.getAssetLockProof(),
        executionContext,
      );
  });
});
