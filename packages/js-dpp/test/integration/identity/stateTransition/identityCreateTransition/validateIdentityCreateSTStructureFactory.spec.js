const getIdentityCreateSTFixture = require('../../../../../lib/test/fixtures/getIdentityCreateSTFixture');

const validateIdentityCreateSTStructureFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/validateIdentityCreateSTStructureFactory',
);

const { expectValidationError } = require(
  '../../../../../lib/test/expect/expectError',
);

const IdentityCreateTransition = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition',
);

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');

describe('validateIdentityCreateSTStructureFactory', () => {
  let validateIdentityCreateST;
  let rawStateTransition;
  let stateTransition;
  let validateIdentityTypeMock;
  let validatePublicKeysMock;

  beforeEach(function beforeEach() {
    validateIdentityTypeMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validatePublicKeysMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateIdentityCreateST = validateIdentityCreateSTStructureFactory(
      validateIdentityTypeMock,
      validatePublicKeysMock,
    );

    stateTransition = getIdentityCreateSTFixture();

    rawStateTransition = stateTransition.toJSON();
  });

  it('should return invalid result if there are duplicate keys', () => {
    const consensusError = new ConsensusError('error');

    validateIdentityTypeMock.returns(new ValidationResult([consensusError]));

    const result = validateIdentityCreateST(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(consensusError);

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawStateTransition.identityType);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });

  it('should return invalid result if identity type is unknown', () => {
    const consensusError = new ConsensusError('error');

    validatePublicKeysMock.returns(new ValidationResult([consensusError]));

    const result = validateIdentityCreateST(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(consensusError);

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawStateTransition.identityType);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });

  it('should pass valid raw state transition', () => {
    const result = validateIdentityCreateST(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawStateTransition.identityType);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });

  it('should pass valid state transition', () => {
    const result = validateIdentityCreateST(new IdentityCreateTransition(rawStateTransition));

    expect(result.isValid()).to.be.true();

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawStateTransition.identityType);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });
});
