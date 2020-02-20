const validateDataContractSTStructureFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractSTStructureFactory');

const DataContractStateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');

const InvalidIdentityPublicKeyTypeError = require('../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');

const Identity = require('../../../../../lib/identity/Identity');

describe('validateDataContractSTStructureFactory', () => {
  let validateDataContract;
  let validateDataContractSTStructure;
  let rawDataContract;
  let rawStateTransition;
  let validateStateTransitionSignatureMock;
  let createDataContractMock;
  let stateTransition;
  let dataContract;
  let validateIdentityExistenceAndTypeMock;

  beforeEach(function beforeEach() {
    validateDataContract = this.sinonSandbox.stub();

    dataContract = getDataContractFixture();

    rawDataContract = dataContract.toJSON();

    stateTransition = new DataContractStateTransition(dataContract);

    rawStateTransition = stateTransition.toJSON();

    createDataContractMock = this.sinonSandbox.stub().returns(dataContract);

    validateStateTransitionSignatureMock = this.sinonSandbox.stub();

    validateIdentityExistenceAndTypeMock = this.sinonSandbox.stub().resolves(
      new ValidationResult(),
    );

    validateDataContractSTStructure = validateDataContractSTStructureFactory(
      validateDataContract,
      validateStateTransitionSignatureMock,
      createDataContractMock,
      validateIdentityExistenceAndTypeMock,
    );
  });

  it('should return invalid result if Data Contract Identity is invalid', async () => {
    const dataContractResult = new ValidationResult();
    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const blockchainUserError = new ConsensusError('error');

    validateIdentityExistenceAndTypeMock.resolves(
      new ValidationResult([blockchainUserError]),
    );

    const result = await validateDataContractSTStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(blockchainUserError);

    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWithExactly(
      dataContract.getId(), [Identity.TYPES.APPLICATION],
    );
  });

  it('should return invalid result if data contract is invalid', async () => {
    const dataContractError = new ConsensusError('test');
    const dataContractResult = new ValidationResult([
      dataContractError,
    ]);

    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractSTStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);

    expect(validateStateTransitionSignatureMock).to.be.not.called();

    expect(validateIdentityExistenceAndTypeMock).to.be.not.called();
  });

  it('should return invalid result on invalid signature', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    const type = 1;
    const validationError = new InvalidIdentityPublicKeyTypeError(type);

    const validateSignatureResult = new ValidationResult([
      validationError,
    ]);

    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractSTStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(validationError);

    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
      stateTransition,
      dataContract.getId(),
    );

    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWithExactly(
      dataContract.getId(), [Identity.TYPES.APPLICATION],
    );
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractSTStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);

    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
      stateTransition,
      dataContract.getId(),
    );

    expect(validateIdentityExistenceAndTypeMock).to.be.calledOnceWithExactly(
      dataContract.getId(), [Identity.TYPES.APPLICATION],
    );
  });
});
