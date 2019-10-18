const validateDataContractSTStructureFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractSTStructureFactory');

const DataContractStateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../../lib/errors/ConsensusError');

describe('validateDataContractSTStructureFactory', () => {
  let validateDataContract;
  let validateDataContractSTStructure;
  let rawDataContract;
  let rawStateTransition;

  beforeEach(function beforeEach() {
    validateDataContract = this.sinonSandbox.stub();

    const dataContract = getDataContractFixture();
    const stateTransition = new DataContractStateTransition(dataContract);

    rawDataContract = dataContract.toJSON();
    rawStateTransition = stateTransition.toJSON();

    validateDataContractSTStructure = validateDataContractSTStructureFactory(
      validateDataContract,
    );
  });

  it('should return invalid result if data contract is not invalid', () => {
    const dataContractError = new ConsensusError('test');
    const dataContractResult = new ValidationResult([
      dataContractError,
    ]);

    validateDataContract.returns(dataContractResult);

    const result = validateDataContractSTStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);
  });

  it('should return valid result', () => {
    const dataContractResult = new ValidationResult();

    validateDataContract.returns(dataContractResult);

    const result = validateDataContractSTStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContract).to.be.calledOnceWith(rawDataContract);
  });
});
