const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion } = require('../../../../..');

describe.skip('applyIdentityCreateTransitionFactory', () => {
  let stateTransition;
  let applyIdentityCreateTransition;
  let stateRepositoryMock;
  let output;
  let executionContext;

  let StateTransitionExecutionContext;
  let Identity;

  let applyIdentityCreateTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      applyIdentityCreateTransition: applyIdentityCreateTransitionDPP,
      Identity,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.createIdentity.resolves();
    stateRepositoryMock.addToSystemCredits.resolves();
    stateRepositoryMock.markAssetLockTransactionOutPointAsUsed.resolves();

    stateTransition = await getIdentityCreateTransitionFixture();

    executionContext = new StateTransitionExecutionContext();

    output = stateTransition.getAssetLockProof().getOutput();

    applyIdentityCreateTransition = (st) => applyIdentityCreateTransitionDPP(
      stateRepositoryMock,
      st,
      executionContext,
    );
  });

  it('should store identity created from state transition', async function beforeEach() {
    await applyIdentityCreateTransition(stateTransition);

    const balance = convertSatoshiToCredits(
      output.satoshis,
    );

    const identity = new Identity({
      protocolVersion: getLatestProtocolVersion(),
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
