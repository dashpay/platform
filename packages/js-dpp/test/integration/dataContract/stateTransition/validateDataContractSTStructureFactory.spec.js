const Ajv = require('ajv');

const validateDataContractSTStructureFactory = require('../../../../lib/dataContract/stateTransition/validateDataContractSTStructureFactory');

const DataContractStateTransition = require('../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');
const dataContractSTSchema = require('../../../../schema/stateTransition/data-contract');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const validateStateTransitionStructureFactory = require('../../../../lib/stateTransition/validate/validateStateTransitionStructureFactory');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError, expectJsonSchemaError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../lib/errors/ConsensusError');

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

  describe('Schema', () => {
    let validateStateTransitionStructure;
    let extensionFunctionMock;

    beforeEach(function beforeEach() {
      extensionFunctionMock = this.sinonSandbox.stub();

      const typeExtensions = {
        [stateTransitionTypes.DATA_CONTRACT]: {
          function: extensionFunctionMock,
          schema: dataContractSTSchema,
        },
      };

      const ajv = new Ajv();
      const validator = new JsonSchemaValidator(ajv);

      validateStateTransitionStructure = validateStateTransitionStructureFactory(
        validator,
        typeExtensions,
      );
    });

    describe('dataContract', () => {
      it('should be present', () => {
        delete rawStateTransition.dataContract;

        const result = validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('dataContract');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });
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
