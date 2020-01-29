const getIdentityCreateSTFixture = require('../../../../../lib/test/fixtures/getIdentityCreateSTFixture');

const { expectValidationError } = require(
  '../../../../../lib/test/expect/expectError',
);

const validateIdentityCreateSTDataFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/validateIdentityCreateSTDataFactory',
);

const IdentityAlreadyExistsError = require(
  '../../../../../lib/errors/IdentityAlreadyExistsError',
);

const createDataProviderMock = require('../../../../../lib/test/mocks/createDataProviderMock');

describe('validateIdentityCreateSTDataFactory', () => {
  let validateIdentityCreateSTData;
  let stateTransition;
  let dataProviderMock;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    validateIdentityCreateSTData = validateIdentityCreateSTDataFactory(dataProviderMock);

    stateTransition = getIdentityCreateSTFixture();
  });

  it('should return invalid result if identity already exists', async () => {
    dataProviderMock.fetchIdentity.resolves({});

    const result = await validateIdentityCreateSTData(stateTransition);

    expectValidationError(result, IdentityAlreadyExistsError, 1);

    const [error] = result.getErrors();

    expect(error.message).to.equal(`Identity with id ${stateTransition.getIdentityId()} already exists`);
    expect(error.getStateTransition()).to.deep.equal(stateTransition);
  });

  it('should return valid result if state transition is valid', async () => {
    const result = await validateIdentityCreateSTData(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
