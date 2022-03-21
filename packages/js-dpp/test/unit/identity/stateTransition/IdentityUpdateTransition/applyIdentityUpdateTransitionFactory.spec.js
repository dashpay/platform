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
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    applyIdentityUpdateTransition = applyIdentityUpdateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should update Identity public keys', async () => {
    await applyIdentityUpdateTransition(stateTransition);

    expect(identity.getPublicKeys()).to.have.lengthOf(3);

    expect(identity.getPublicKeyById(3).toObject())
      .to.deep.equal(stateTransition.getAddPublicKeys()[0]);

    const [id] = stateTransition.getDisablePublicKeys();

    expect(identity.getPublicKeyById(id).getDisabledAt())
      .to.equal(stateTransition.getPublicKeysDisabledAt());
  });
});
