const validateStateTransitionBasicFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionBasicFactory');

const DataContractFactory = require('../../../../lib/dataContract/DataContractFactory');

const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const StateTransitionMaxSizeExceededError = require('../../../../lib/errors/consensus/basic/stateTransition/StateTransitionMaxSizeExceededError');

const {
  expectValidationError,
} = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../lib/errors/consensus/ConsensusError');
const MissingStateTransitionTypeError = require('../../../../lib/errors/consensus/basic/stateTransition/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../../../lib/errors/consensus/basic/stateTransition/InvalidStateTransitionTypeError');
const createDPPMock = require('../../../../lib/test/mocks/createDPPMock');

describe('validateStateTransitionBasicFactory', () => {
  let validateStateTransitionBasic;
  let validationFunctionMock;
  let rawStateTransition;
  let dataContract;
  let dataContractFactory;
  let createStateTransitionMock;
  let stateTransition;

  beforeEach(function beforeEach() {
    validationFunctionMock = this.sinonSandbox.stub();

    const validationFunctionsByType = {
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validationFunctionMock,
    };

    dataContract = getDataContractFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    dataContractFactory = new DataContractFactory(createDPPMock(), undefined);

    stateTransition = dataContractFactory.createStateTransition(dataContract);
    stateTransition.signByPrivateKey(privateKey);

    rawStateTransition = stateTransition.toObject();

    createStateTransitionMock = this.sinonSandbox.stub().resolves(stateTransition);

    validateStateTransitionBasic = validateStateTransitionBasicFactory(
      validationFunctionsByType,
      createStateTransitionMock,
    );
  });

  it('should return invalid result if ST type is missing', async () => {
    delete rawStateTransition.type;

    const result = await validateStateTransitionBasic(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceof(MissingStateTransitionTypeError);

    expect(validationFunctionMock).to.not.be.called();
  });

  it('should return invalid result if ST type is not valid', async () => {
    rawStateTransition.type = 666;

    const result = await validateStateTransitionBasic(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceof(InvalidStateTransitionTypeError);

    expect(validationFunctionMock).to.not.be.called();
  });

  it('should return invalid result if ST is invalid against validation function', async () => {
    const extensionError = new ConsensusError('test');
    const extensionResult = new ValidationResult([
      extensionError,
    ]);

    validationFunctionMock.returns(extensionResult);

    const result = await validateStateTransitionBasic(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(extensionError);

    expect(validationFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });

  it('should return invalid result if ST size is more than 16 kb', async () => {
    const validationFunctionResult = new ValidationResult();

    validationFunctionMock.returns(validationFunctionResult);

    // generate big state transition
    for (let i = 0; i < 500; i++) {
      stateTransition.dataContract.documents[`anotherContract${i}`] = rawStateTransition.dataContract.documents.niceDocument;
    }

    const result = await validateStateTransitionBasic(
      rawStateTransition,
    );

    expectValidationError(result, StateTransitionMaxSizeExceededError);
  });

  it('should return valid result', async () => {
    const extensionResult = new ValidationResult();

    validationFunctionMock.returns(extensionResult);

    const result = await validateStateTransitionBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validationFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });
});
