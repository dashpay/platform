const validateDataContractCreateTransitionStructureFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractCreateTransitionStructureFactory');

const DataContractCreateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractCreateTransition');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');

const InvalidIdentityPublicKeyTypeError = require('../../../../../lib/errors/InvalidIdentityPublicKeyTypeError');
const InvalidDataContractIdError = require('../../../../../lib/errors/InvalidDataContractIdError');
const InvalidDataContractEntropyError = require('../../../../../lib/errors/InvalidDataContractEntropyError');

describe('validateDataContractCreateTransitionStructureFactory', () => {
  let validateDataContract;
  let validateDataContractCreateTransitionStructure;
  let rawDataContract;
  let rawStateTransition;
  let validateStateTransitionSignatureMock;
  let stateTransition;
  let dataContract;
  let validateIdentityExistenceMock;

  beforeEach(function beforeEach() {
    validateDataContract = this.sinonSandbox.stub();

    dataContract = getDataContractFixture();

    rawDataContract = dataContract.toJSON();

    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });

    rawStateTransition = stateTransition.toJSON();

    validateStateTransitionSignatureMock = this.sinonSandbox.stub();

    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(
      new ValidationResult(),
    );

    // eslint-disable-next-line max-len
    validateDataContractCreateTransitionStructure = validateDataContractCreateTransitionStructureFactory(
      validateDataContract,
      validateStateTransitionSignatureMock,
      validateIdentityExistenceMock,
    );
  });

  it('should return invalid result if Data Contract Identity is invalid', async () => {
    const dataContractResult = new ValidationResult();
    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const blockchainUserError = new ConsensusError('error');

    validateIdentityExistenceMock.resolves(
      new ValidationResult([blockchainUserError]),
    );

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(blockchainUserError);

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
      dataContract.getOwnerId(),
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

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);

    expect(validateStateTransitionSignatureMock).to.be.not.called();

    expect(validateIdentityExistenceMock).to.be.not.called();
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

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(validationError);

    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
      stateTransition,
      dataContract.getOwnerId(),
    );

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
      dataContract.getOwnerId(),
    );
  });

  it('should return invalid result on invalid Data Contract id', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    rawStateTransition.dataContract.$id = '5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq';

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(InvalidDataContractIdError);
    expect(error.getRawDataContract()).to.equal(rawStateTransition.dataContract);
  });

  it('should return invalid result on invalid entropy', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    rawStateTransition.entropy = '86b273ff86b273ff86b273ff86b273ff86';

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(InvalidDataContractEntropyError);
    expect(error.getRawDataContract()).to.deep.equal(rawStateTransition.dataContract);
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    const validateSignatureResult = new ValidationResult();
    validateStateTransitionSignatureMock.resolves(validateSignatureResult);

    const result = await validateDataContractCreateTransitionStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);

    expect(validateStateTransitionSignatureMock).to.be.calledOnceWith(
      stateTransition,
      dataContract.getOwnerId(),
    );

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(
      dataContract.getOwnerId(),
    );
  });
});
