const { expectValidationError } = require(
  '../../../../../../../lib/test/expect/expectError',
);

const validateIdentityCreateTransitionStateFactory = require(
  '../../../../../../../lib/identity/stateTransition/IdentityCreateTransition/validation/state/validateIdentityCreateTransitionStateFactory',
);

const getIdentityCreateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const IdentityAlreadyExistsError = require(
  '../../../../../../../lib/errors/IdentityAlreadyExistsError',
);

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

describe('validateIdentityCreateTransitionStateFactory', () => {
  let validateIdentityCreateTransitionState;
  let stateTransition;
  let stateRepositoryMock;
  let validateIdentityPublicKeyUniquenessMock;

  beforeEach(function beforeEach() {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    validateIdentityPublicKeyUniquenessMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityCreateTransitionState = validateIdentityCreateTransitionStateFactory(
      stateRepositoryMock,
      validateIdentityPublicKeyUniquenessMock,
    );

    stateTransition = getIdentityCreateTransitionFixture();

    stateTransition.signByPrivateKey(privateKey);

    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock.fetchTransaction.resolves(rawTransaction);
  });

  it('should return invalid result if identity already exists', async () => {
    stateRepositoryMock.fetchIdentity.resolves({});

    const result = await validateIdentityCreateTransitionState(stateTransition);

    expectValidationError(result, IdentityAlreadyExistsError, 1);

    const [error] = result.getErrors();

    expect(error.message).to.equal(`Identity with id ${stateTransition.getIdentityId()} already exists`);
    expect(error.getStateTransition()).to.deep.equal(stateTransition);
  });

  it('should return invalid result if identity public key already exists', async () => {
    const validationError = new Error('Some error');

    const validationResult = new ValidationResult([
      validationError,
    ]);

    validateIdentityPublicKeyUniquenessMock.returns(validationResult);

    const result = await validateIdentityCreateTransitionState(stateTransition);

    const [error] = result.getErrors();
    expect(error).to.deep.equal(validationError);
  });

  it('should return valid result if state transition is valid', async () => {
    const result = await validateIdentityCreateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
