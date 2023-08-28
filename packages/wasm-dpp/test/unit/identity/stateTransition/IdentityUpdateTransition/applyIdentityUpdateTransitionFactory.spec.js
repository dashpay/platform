const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe.skip('applyIdentityUpdateTransition', () => {
  let applyIdentityUpdateTransition;
  let stateRepositoryMock;
  let stateTransition;
  let executionContext;

  let StateTransitionExecutionContext;

  let applyIdentityUpdateTransitionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      applyIdentityUpdateTransition: applyIdentityUpdateTransitionDPP,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateTransition = await getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(stateTransition.getRevision() + 1);

    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.updateIdentityRevision.resolves();
    stateRepositoryMock.disableIdentityKeys.resolves();
    stateRepositoryMock.addKeysToIdentity.resolves();

    applyIdentityUpdateTransition = (st) => applyIdentityUpdateTransitionDPP(
      stateRepositoryMock,
      st,
      executionContext,
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
      .map((publicKey) => publicKey.toObject({ skipSignature: true }));

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      match((args) => expect(args.map((k) => k.toObject())).to.deep.equals(publicKeysToAdd)),
      match.instanceOf(StateTransitionExecutionContext),
    );
  });

  it('should add and disable public keys on dry run', async function () {
    const { match } = this.sinonSandbox;

    executionContext.enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    executionContext.disableDryRun();

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
      .map((publicKey) => publicKey.toObject({ skipSignature: true }));

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      match((args) => expect(args.map((k) => k.toObject())).to.deep.equals(publicKeysToAdd)),
      match.instanceOf(StateTransitionExecutionContext),
    );
  });
});
