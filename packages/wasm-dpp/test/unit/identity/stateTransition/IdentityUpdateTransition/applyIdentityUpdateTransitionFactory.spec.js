const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityUpdateTransitionFixture');
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
    stateRepositoryMock.fetchIdentity.resolves(identity);

    applyIdentityUpdateTransition = (st) => applyIdentityUpdateTransitionDPP(
      stateRepositoryMock,
      st,
    );
  });

  it('should add public keys', async function () {
    stateTransition.setPublicKeysDisabledAt(undefined);
    stateTransition.setPublicKeyIdsToDisable(undefined);

    await applyIdentityUpdateTransition(stateTransition);

    const { args: [updatedIdentity] } = stateRepositoryMock.updateIdentity.firstCall;

    expect(updatedIdentity.getPublicKeys()).to.have.lengthOf(3);

    expect(updatedIdentity.getPublicKeyById(3).toObject())
      .to.deep.equal(stateTransition.getPublicKeysToAdd()[0].toObject());

    const { match } = this.sinonSandbox;
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      updatedIdentity,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const publicKeyHashes = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(updatedIdentity.getId().toBuffer())),
      match((hashes) => expect(hashes).to.deep.equal(publicKeyHashes)),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(updatedIdentity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should disable public key', async function () {
    stateTransition.setPublicKeysToAdd(undefined);

    await applyIdentityUpdateTransition(stateTransition);

    const { args: [updatedIdentity] } = stateRepositoryMock.updateIdentity.firstCall;

    const { match } = this.sinonSandbox;
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      updatedIdentity,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const [id] = stateTransition.getPublicKeyIdsToDisable();

    expect(updatedIdentity.getPublicKeyById(id).getDisabledAt())
      .to.equal(stateTransition.getPublicKeysDisabledAt().getTime());

    expect(updatedIdentity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should not add public keys on dry run', async function () {
    stateTransition.setPublicKeysDisabledAt(undefined);
    stateTransition.setPublicKeyIdsToDisable(undefined);

    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    const { args: [updatedIdentity] } = stateRepositoryMock.updateIdentity.firstCall;

    expect(updatedIdentity.getPublicKeys()).to.have.lengthOf(11);

    const { match } = this.sinonSandbox;
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      updatedIdentity,
      match.instanceOf(StateTransitionExecutionContext),
    );

    const publicKeyHashes = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(updatedIdentity.getId().toBuffer())),
      match((hashes) => expect(hashes).to.deep.equal(publicKeyHashes)),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(updatedIdentity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should use biggestPossibleIdentity on dry run', async function () {
    const biggestPossibleBalance = 18446744073709552000;

    stateTransition.setPublicKeysToAdd(undefined);

    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    const { args: [biggestPossibleIdentity] } = stateRepositoryMock.updateIdentity.firstCall;
    expect(biggestPossibleIdentity.getBalance()).to.be.equal(biggestPossibleBalance);

    const { match } = this.sinonSandbox;
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      match((id) => id.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
      match.instanceOf(StateTransitionExecutionContext),
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      biggestPossibleIdentity,
      match.instanceOf(StateTransitionExecutionContext),
    );
  });
});
