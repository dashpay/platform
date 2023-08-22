const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../..');

describe.skip('applyIdentityTopUpTransitionFactory', () => {
  let stateTransition;
  let applyIdentityTopUpTransition;
  let stateRepositoryMock;
  let executionContext;

  let StateTransitionExecutionContext;

  let applyIdentityTopUpTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      applyIdentityTopUpTransition: applyIdentityTopUpTransitionDPP,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(0);
    stateRepositoryMock.addToIdentityBalance.resolves();
    stateRepositoryMock.addToSystemCredits.resolves();
    stateRepositoryMock.markAssetLockTransactionOutPointAsUsed.resolves();

    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock.fetchTransaction.resolves({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    stateTransition = await getIdentityTopUpTransitionFixture();

    executionContext = new StateTransitionExecutionContext();

    applyIdentityTopUpTransition = (st) => applyIdentityTopUpTransitionDPP(
      stateRepositoryMock,
      st,
      executionContext,
    );
  });

  it('should add topup amount to identity balance', async function () {
    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

    const { match } = this.sinonSandbox;

    expect(stateRepositoryMock.addToIdentityBalance).to.have.been.calledOnceWithExactly(
      match((arg) => arg.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      balanceToTopUp,
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.addToSystemCredits).to.have.been.calledOnceWithExactly(
      balanceToTopUp,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        match((arg) => Buffer.from(arg).equals(outPoint)),
        match.instanceOf(StateTransitionExecutionContext),
      );
  });

  it('should ignore balance debt for system credits', async function () {
    stateRepositoryMock.fetchIdentityBalanceWithDebt.resolves(-5);

    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

    const { match } = this.sinonSandbox;

    expect(stateRepositoryMock.addToIdentityBalance).to.have.been.calledOnceWithExactly(
      match((arg) => arg.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      balanceToTopUp,
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.addToSystemCredits).to.have.been.calledOnceWithExactly(
      balanceToTopUp - 5,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        match((arg) => Buffer.from(arg).equals(outPoint)),
        match.instanceOf(StateTransitionExecutionContext),
      );
  });

  it('should add topup amount to identity balance on dry run', async function () {
    const { match } = this.sinonSandbox;

    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    executionContext.enableDryRun();

    await applyIdentityTopUpTransition(stateTransition);

    executionContext.disableDryRun();

    expect(stateRepositoryMock.addToIdentityBalance).to.have.been.calledOnceWithExactly(
      match((arg) => arg.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      balanceToTopUp,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        match((arg) => Buffer.from(arg).equals(outPoint)),
        match.instanceOf(StateTransitionExecutionContext),
      );
  });
});
