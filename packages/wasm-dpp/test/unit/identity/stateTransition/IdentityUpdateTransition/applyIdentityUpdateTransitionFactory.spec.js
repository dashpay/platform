const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('applyIdentityUpdateTransition', () => {
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
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.updateIdentityRevision.resolves();
    stateRepositoryMock.disableIdentityKeys.resolves();
    stateRepositoryMock.addKeysToIdentity.resolves();

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
      .map((publicKey) => publicKey.toObject({ skipSignature: true }));

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getOwnerId().toBuffer())),
      match((args) => expect(args.map((k) => k.toObject())).to.deep.equals(publicKeysToAdd)),
      match.instanceOf(StateTransitionExecutionContext),
    );
  });

  it('should add and disable public keys on dry run', async function () {
    const { match } = this.sinonSandbox;

    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

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
