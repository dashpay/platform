const IdentityCreateTransition = require('../../../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const stateTransitionTypes = require('../../../../../lib/stateTransition/stateTransitionTypes');

const { expectValidationError } = require(
  '../../../../../lib/test/expect/expectError',
);

const validateIdentityCreateSTDataFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/validateIdentityCreateSTDataFactory',
);

const IdentityAlreadyExistsError = require(
  '../../../../../lib/errors/IdentityAlreadyExistsError',
);

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

describe('validateIdentityCreateSTDataFactory', () => {
  let validateIdentityCreateSTData;
  let stateTransition;
  let stateRepositoryMock;
  let validateAssetLockTransactionMock;
  let validateIdentityPublicKeyUniquenessMock;

  beforeEach(function beforeEach() {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    validateAssetLockTransactionMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validateIdentityPublicKeyUniquenessMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    validateIdentityCreateSTData = validateIdentityCreateSTDataFactory(
      stateRepositoryMock,
      validateAssetLockTransactionMock,
      validateIdentityPublicKeyUniquenessMock,
    );

    stateTransition = new IdentityCreateTransition({
      protocolVersion: 0,
      type: stateTransitionTypes.IDENTITY_CREATE,
      lockedOutPoint: 'azW1UgBiB0CmdphN6of4DbT91t0Xv3/c3YUV4CnoV/kAAAAA',
      publicKeys: [
        {
          id: 0,
          type: 1,
          data: 'Alw8x/v8UvcQyUFJf9AYdsGJFx6iJ0WPUBr8s4opfWW0',
          isEnabled: true,
        },
      ],
    });
    stateTransition.signByPrivateKey(privateKey);

    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock.fetchTransaction.resolves(rawTransaction);
  });

  it('should return invalid result if identity already exists', async () => {
    stateRepositoryMock.fetchIdentity.resolves({});

    const result = await validateIdentityCreateSTData(stateTransition);

    expectValidationError(result, IdentityAlreadyExistsError, 1);

    const [error] = result.getErrors();

    expect(error.message).to.equal(`Identity with id ${stateTransition.getIdentityId()} already exists`);
    expect(error.getStateTransition()).to.deep.equal(stateTransition);
  });

  it('should return invalid result if lock transaction is invalid', async () => {
    const validationError = new Error('Some error');

    const validationResult = new ValidationResult([
      validationError,
    ]);

    validateAssetLockTransactionMock.returns(validationResult);

    const result = await validateIdentityCreateSTData(stateTransition);

    const [error] = result.getErrors();
    expect(error).to.deep.equal(validationError);
  });

  it('should return invalid result if identity public key already exists', async () => {
    const validationError = new Error('Some error');

    const validationResult = new ValidationResult([
      validationError,
    ]);

    validateIdentityPublicKeyUniquenessMock.returns(validationResult);

    const result = await validateIdentityCreateSTData(stateTransition);

    const [error] = result.getErrors();
    expect(error).to.deep.equal(validationError);
  });

  it('should return valid result if state transition is valid', async () => {
    const result = await validateIdentityCreateSTData(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
