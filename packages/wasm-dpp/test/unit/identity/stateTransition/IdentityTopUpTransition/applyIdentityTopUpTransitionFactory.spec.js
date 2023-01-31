const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');

const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../../dist');
const generateRandomIdentifierAsync = require('../../../../../lib/test/utils/generateRandomIdentifierAsync');

describe('applyIdentityTopUpTransitionFactory', () => {
  let stateTransition;
  let applyIdentityTopUpTransition;
  let stateRepositoryMock;
  let identity;
  let executionContext;

  let StateTransitionExecutionContext;
  let IdentityTopUpTransition;
  let Identity;

  let applyIdentityTopUpTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      IdentityTopUpTransition,
      applyIdentityTopUpTransition: applyIdentityTopUpTransitionDPP,
      Identity,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const rawIdentity = getIdentityFixture().toObject();
    // Patch identity id to match expectation of wasm Identity class
    rawIdentity.id = await generateRandomIdentifierAsync();
    identity = new Identity(rawIdentity);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.returns(identity);

    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock.fetchTransaction.returns({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    stateTransition = new IdentityTopUpTransition(
      getIdentityTopUpTransitionFixture().toObject(),
    );

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    applyIdentityTopUpTransition = (st) => applyIdentityTopUpTransitionDPP(
      stateRepositoryMock,
      st,
    );
  });

  it('should store identity created from state transition', async function () {
    const balanceBeforeTopUp = identity.getBalance();

    const balanceToTopUp = convertSatoshiToCredits(
      stateTransition.getAssetLockProof().getOutput().satoshis,
    );

    await applyIdentityTopUpTransition(stateTransition);

    const { args: [updatedIdentity] } = stateRepositoryMock.updateIdentity.firstCall;

    expect(updatedIdentity.getBalance()).to.be.equal(balanceBeforeTopUp + balanceToTopUp);
    expect(updatedIdentity.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

    expect(stateRepositoryMock.updateIdentity).to.have.been.calledOnceWithExactly(
      updatedIdentity,
      this.sinonSandbox.match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
      );
  });

  it('should store biggest possible identity on dry run', async function () {
    const biggestPossibleBalance = 18446744073709552000;

    executionContext.enableDryRun();

    await applyIdentityTopUpTransition(stateTransition);

    executionContext.disableDryRun();

    const { args: [biggestPossibleIdentity] } = stateRepositoryMock.updateIdentity.firstCall;
    expect(biggestPossibleIdentity.getBalance()).to.be.equal(biggestPossibleBalance);

    expect(stateRepositoryMock.updateIdentity).to.have.been.calledOnceWithExactly(
      biggestPossibleIdentity,
      this.sinonSandbox.match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.markAssetLockTransactionOutPointAsUsed).to.have.been
      .calledOnceWithExactly(
        stateTransition.getAssetLockProof().getOutPoint(),
      );
  });
});
