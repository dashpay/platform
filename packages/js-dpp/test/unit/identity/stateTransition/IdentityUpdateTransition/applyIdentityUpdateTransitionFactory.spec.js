const applyIdentityUpdateTransitionFactory = require('../../../../../lib/identity/stateTransition/IdentityUpdateTransition/applyIdentityUpdateTransitionFactory');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const getIdentityFixture = require('../../../../../lib/test/fixtures/getIdentityFixture');

describe('applyIdentityUpdateTransition', () => {
  let applyIdentityUpdateTransition;
  let stateRepositoryMock;
  let stateTransition;
  let identity;

  beforeEach(function beforeEach() {
    stateTransition = getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(stateTransition.getRevision() + 1);
    identity = getIdentityFixture();

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
    );

    expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWithExactly(
      identity,
    );

    const publicKeyHashes = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(identity.getRevision()).to.equal(stateTransition.getRevision());
  });

  it('should disable public key', async () => {
    stateTransition.setPublicKeysToAdd(undefined);

    await applyIdentityUpdateTransition(stateTransition);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();

    expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWithExactly(
      identity,
    );

    const [id] = stateTransition.getPublicKeyIdsToDisable();

    expect(identity.getPublicKeyById(id).getDisabledAt())
      .to.equal(stateTransition.getPublicKeysDisabledAt().getTime());

    expect(identity.getRevision()).to.equal(stateTransition.getRevision());
  });
});
