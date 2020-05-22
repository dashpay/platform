const validateIdentityPublicKeyUniquenessFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/validateIdentityPublicKeyUniquenessFactory',
);

const IdentityFirstPublicKeyAlreadyExistsError = require(
  '../../../../../lib/errors/IdentityFirstPublicKeyAlreadyExistsError',
);

const getIdentityFixture = require('../../../../../lib/test/fixtures/getIdentityFixture');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const { expectValidationError } = require(
  '../../../../../lib/test/expect/expectError',
);

describe('validateIdentityPublicKeyUniquenessFactory', () => {
  let identity;
  let stateRepositoryMock;
  let validateIdentityPublicKeyUniqueness;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    validateIdentityPublicKeyUniqueness = validateIdentityPublicKeyUniquenessFactory(
      stateRepositoryMock,
    );
  });

  it('should return invalid result if identity id was found', async () => {
    stateRepositoryMock.fetchPublicKeyIdentityId.resolves(identity.getId());

    const [firstPublicKey] = identity.getPublicKeys();
    const result = await validateIdentityPublicKeyUniqueness(
      firstPublicKey,
    );

    expectValidationError(result, IdentityFirstPublicKeyAlreadyExistsError);

    const [error] = result.getErrors();

    expect(error.getPublicKeyHash()).to.equal(identity.getPublicKeyById(0).hash());
  });

  it('should return valid result if no identity id was not found', async () => {
    const result = await validateIdentityPublicKeyUniqueness(
      identity.getPublicKeyById(0),
    );

    expect(result.isValid()).to.be.true();
  });
});
