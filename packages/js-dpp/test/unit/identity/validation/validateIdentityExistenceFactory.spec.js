const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');

const validateIdentityExistenceFactory = require('../../../../lib/identity/validation/validateIdentityExistenceFactory');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const IdentityNotFoundError = require('../../../../lib/errors/consensus/signature/IdentityNotFoundError');

describe('validateIdentityExistence', () => {
  let validateIdentityExistence;
  let stateRepositoryMock;
  let identity;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    validateIdentityExistence = validateIdentityExistenceFactory(
      stateRepositoryMock,
    );

    identity = getIdentityFixture();
  });

  it('should return invalid result if identity is not found', async () => {
    const result = await validateIdentityExistence(identity.getId());

    expectValidationError(result, IdentityNotFoundError);

    const [error] = result.getErrors();

    expect(error.getIdentityId()).to.equal(identity.getId());
  });

  it('should return valid result', async () => {
    stateRepositoryMock.fetchIdentity.resolves(identity);

    const result = await validateIdentityExistence(identity.getId());

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
