const applyIdentityUpdateTransitionFactory = require('../../../../../lib/identity/stateTransition/IdentityUpdateTransition/applyIdentityUpdateTransitionFactory');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const getIdentityFixture = require('../../../../../lib/test/fixtures/getIdentityFixture');
const StateTransitionExecutionContext = require('../../../../../lib/stateTransition/StateTransitionExecutionContext');
const getBiggestPossibleIdentity = require('../../../../../lib/identity/getBiggestPossibleIdentity');

describe('applyIdentityUpdateTransition', () => {
  let applyIdentityUpdateTransition;
  let stateRepositoryMock;
  let stateTransition;
  let identity;
  let executionContext;

  beforeEach(function beforeEach() {
    stateTransition = getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(stateTransition.getRevision() + 1);
    identity = getIdentityFixture();

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    applyIdentityUpdateTransition = applyIdentityUpdateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should add public keys', async () => {
    stateTransition.setPublicKeysDisabledAt(undefined);
    stateTransition.setPublicKeyIdsToDisable(undefined);

    await applyIdentityUpdateTransition(stateTransition);

    expect(identity.getPublicKeys()).to.have.lengthOf(3);

    expect(identity.getPublicKeyById(3).toObject())
      .to.deep.equal(stateTransition.getPublicKeysToAdd()[0]);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
      executionContext,
    );

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      identity,
      executionContext,
    );

    const publicKeyHashes = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
      executionContext,
    );

    expect(identity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should disable public key', async () => {
    stateTransition.setPublicKeysToAdd(undefined);

    await applyIdentityUpdateTransition(stateTransition);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
      executionContext,
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      identity,
      executionContext,
    );

    const [id] = stateTransition.getPublicKeyIdsToDisable();

    expect(identity.getPublicKeyById(id).getDisabledAt())
      .to.equal(stateTransition.getPublicKeysDisabledAt().getTime());

    expect(identity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should not add public keys on dry run', async () => {
    const biggestPossibleIdentity = getBiggestPossibleIdentity();

    stateTransition.setPublicKeysDisabledAt(undefined);
    stateTransition.setPublicKeyIdsToDisable(undefined);

    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(identity.getPublicKeys()).to.have.lengthOf(2);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
      executionContext,
    );

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      biggestPossibleIdentity,
      executionContext,
    );

    const publicKeyHashes = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      biggestPossibleIdentity.getId(),
      publicKeyHashes,
      executionContext,
    );

    expect(biggestPossibleIdentity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should use biggestPossibleIdentity on dry run', async () => {
    const biggestPossibleIdentity = getBiggestPossibleIdentity();

    stateTransition.setPublicKeysToAdd(undefined);

    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
      executionContext,
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(
      biggestPossibleIdentity,
      executionContext,
    );
  });
});
