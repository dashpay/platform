const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const { convertSatoshiToCredits } = require('@dashevo/dpp/lib/identity/creditsConverter');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../dist');

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

    const stateTransitionJS = getIdentityCreateTransitionFixture();
    stateTransition = new IdentityCreateTransition(stateTransitionJS.toObject());

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    output = stateTransition.getAssetLockProof().getOutput();

    applyIdentityCreateTransition = (st) => applyIdentityCreateTransitionDPP(
      stateRepositoryMock,
      st,
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

    const { args: createIdentityArgs } = stateRepositoryMock.createIdentity.firstCall;

    expect(createIdentityArgs[0].toObject()).to.deep.equal(identity.toObject());
    expect(createIdentityArgs[1]).to.be.instanceOf(StateTransitionExecutionContext);

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    const { args: storePublicKeyHashesArgs } = stateRepositoryMock
      .storeIdentityPublicKeyHashes.firstCall;
    expect(storePublicKeyHashesArgs[0].toBuffer()).to.deep.equal(identity.getId().toBuffer());
    expect(storePublicKeyHashesArgs[1]).to.deep.equal(publicKeyHashes);
    expect(storePublicKeyHashesArgs[2]).to.be.instanceOf(StateTransitionExecutionContext);

    const { args: markAsUsedArgs } = stateRepositoryMock
      .markAssetLockTransactionOutPointAsUsed.firstCall;

    expect(markAsUsedArgs[0]).to
      .deep.equal(stateTransition.getAssetLockProof().getOutPoint());
  });
});
