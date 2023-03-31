const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../..');

describe('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let stateRepositoryMock;
  let output;
  let executionContext;

  let StateTransitionExecutionContext;
  let IdentityCreateTransition;
  let Identity;

  let applyIdentityCreateTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      IdentityCreateTransition,
      applyIdentityCreateTransition: applyIdentityCreateTransitionDPP,
      Identity,
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.createIdentity.resolves();
    stateRepositoryMock.addToSystemCredits.resolves();
    stateRepositoryMock.markAssetLockTransactionOutPointAsUsed.resolves();

    stateTransition = new IdentityCreateTransition(
      getIdentityCreateTransitionFixture().toObject(),
    );

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    output = stateTransition.getAssetLockProof().getOutput();

    applyIdentityCreateTransition = (st) => applyIdentityCreateTransitionDPP(
      stateRepositoryMock,
      st,
    );
  });

  it('should store identity created from state transition', async function () {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(
      output.satoshis,
    );

    const identity = new Identity({
      protocolVersion: protocolVersion.latestVersion,
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys()
        .map((key) => key.toObject({ skipSignature: true })),
      balance,
      revision: 0,
    });

    const { match } = this.sinonSandbox;

    expect(stateRepositoryMock.createIdentity).to.have.been.calledOnceWithExactly(
      match((arg) => expect(arg.toObject()).to.deep.equal(identity.toObject())),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.addToSystemCredits).to.have.been.calledOnceWithExactly(
      balance,
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
