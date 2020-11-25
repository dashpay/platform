const generateRandomIdentifier = require('../../../../lib/test/utils/generateRandomIdentifier');

const IdentityPublicKey = require('../../../../lib/identity/IdentityPublicKey');

const validateIdentityExistenceFactory = require('../../../../lib/identity/validation/validateIdentityExistenceFactory');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const IdentityNotFoundError = require('../../../../lib/errors/IdentityNotFoundError');

describe('validateIdentityExistence', () => {
  let validateIdentityExistence;
  let stateRepositoryMock;
  let ownerId;
  let rawIdentityUser;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    validateIdentityExistence = validateIdentityExistenceFactory(
      stateRepositoryMock,
    );

    ownerId = generateRandomIdentifier();

    rawIdentityUser = {
      id: ownerId,
      publicKeys: [
        {
          id: 0,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.from('z3HAPrJkpgffXX0b3w0lb/PZs6A5IXzHj1p8Fnzmgmk=', 'base64'),
        },
      ],
    };
  });

  it('should return invalid result if identity is not found', async () => {
    const result = await validateIdentityExistence(ownerId);

    expectValidationError(result, IdentityNotFoundError);

    const [error] = result.getErrors();

    expect(error.getIdentityId()).to.equal(ownerId);
  });

  it('should return valid result', async () => {
    stateRepositoryMock.fetchIdentity.resolves(rawIdentityUser);

    const result = await validateIdentityExistence(ownerId);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
