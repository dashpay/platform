const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityUpdateTransitionFixture');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const { default: loadWasmDpp } = require('../../../../../dist');
const generateRandomIdentifierAsync = require('../../../../../lib/test/utils/generateRandomIdentifierAsync');

describe('applyIdentityUpdateTransition', () => {
  let applyIdentityUpdateTransition;
  let stateRepositoryMock;
  let stateTransition;
  let executionContext;

  let StateTransitionExecutionContext;
  let IdentityUpdateTransition;
  let Identity;

  let applyIdentityUpdateTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      IdentityUpdateTransition,
      applyIdentityUpdateTransition: applyIdentityUpdateTransitionDPP,
      Identity,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateTransition = new IdentityUpdateTransition(
      getIdentityUpdateTransitionFixture().toObject(),
    );
    stateTransition.setRevision(stateTransition.getRevision() + 1);

    const rawIdentity = getIdentityFixture().toObject();
    // Patch identity id to match expectation of wasm Identity class
    rawIdentity.id = await generateRandomIdentifierAsync();
    const identity = new Identity(rawIdentity);

    executionContext = new StateTransitionExecutionContext();
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    applyIdentityUpdateTransition = (st) => applyIdentityUpdateTransitionDPP(
      stateRepositoryMock,
      st,
    );
  });

  it('should add and disable public keys', async function () {
    await applyIdentityUpdateTransition(stateTransition);

    const { match } = this.sinonSandbox;

    expect(stateRepositoryMock.updateIdentityRevision).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      stateTransition.getRevision(),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.disableIdentityKeys).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      stateTransition.getPublicKeyIdsToDisable(),
      stateTransition.getPublicKeysDisabledAt().getTime(),
      match.instanceOf(StateTransitionExecutionContext),
    );

    const publicKeysToAdd = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => {
        const rawPublicKey = publicKey.toObject({ skipSignature: true });

        return new IdentityPublicKey(rawPublicKey);
      });

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      publicKeysToAdd,
      match.instanceOf(StateTransitionExecutionContext),
    );
  });

  it('should add and disable public keys on dry run', async () => {
    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(stateRepositoryMock.updateIdentityRevision).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getRevision(),
      executionContext,
    );

    expect(stateRepositoryMock.disableIdentityKeys).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getPublicKeyIdsToDisable(),
      stateTransition.getPublicKeysDisabledAt().getTime(),
      executionContext,
    );

    const publicKeysToAdd = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => {
        const rawPublicKey = publicKey.toObject({ skipSignature: true });

        return new IdentityPublicKey(rawPublicKey);
      });

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      publicKeysToAdd,
      executionContext,
    );
  });
});
