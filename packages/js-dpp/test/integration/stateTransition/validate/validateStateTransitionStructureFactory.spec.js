const Ajv = require('ajv');

const validateStateTransitionStructureFactory = require('../../../../lib/stateTransition/validate/validateStateTransitionStructureFactory');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');

const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const ConsensusError = require('../../../../lib/errors/ConsensusError');
const MissingStateTransitionTypeError = require('../../../../lib/errors/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../../../lib/errors/InvalidStateTransitionTypeError');

describe('validateStateTransitionStructureFactory', () => {
  let validateStateTransitionStructure;
  let extensionFunctionMock;
  let rawStateTransition;

  beforeEach(function beforeEach() {
    extensionFunctionMock = this.sinonSandbox.stub();

    const extensionSchema = {
      properties: {
        extension: {
          type: 'object',
        },
      },
      required: ['extension'],
    };

    const typeExtensions = {
      [stateTransitionTypes.DATA_CONTRACT]: {
        function: extensionFunctionMock,
        schema: extensionSchema,
      },
    };

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateStateTransitionStructure = validateStateTransitionStructureFactory(
      validator,
      typeExtensions,
    );

    rawStateTransition = {
      protocolVersion: 0,
      type: stateTransitionTypes.DATA_CONTRACT,
      extension: {},
    };
  });

  describe('Base schema', () => {
    describe('protocolVersion', () => {
      it('should be present', () => {
        delete rawStateTransition.protocolVersion;

        const result = validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('protocolVersion');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should equal to 0', () => {
        rawStateTransition.protocolVersion = 666;

        const result = validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.protocolVersion');
        expect(error.keyword).to.equal('const');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('type', () => {
      it('should be present', () => {
        delete rawStateTransition.type;

        const result = validateStateTransitionStructure(rawStateTransition);

        expectValidationError(
          result,
          MissingStateTransitionTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawStateTransition()).to.equal(rawStateTransition);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have defined extension', () => {
        rawStateTransition.type = 666;

        const result = validateStateTransitionStructure(rawStateTransition);

        expectValidationError(
          result,
          InvalidStateTransitionTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawStateTransition()).to.equal(rawStateTransition);

        expect(extensionFunctionMock).to.not.be.called();
      });
    });
  });

  it('should return invalid result if ST invalid against extension schema', () => {
    delete rawStateTransition.extension;

    const result = validateStateTransitionStructure(rawStateTransition);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('required');
    expect(error.params.missingProperty).to.equal('extension');

    expect(extensionFunctionMock).to.not.be.called();
  });

  it('should return invalid result if ST is invalid against extension function', () => {
    const extensionError = new ConsensusError('test');
    const extensionResult = new ValidationResult([
      extensionError,
    ]);

    extensionFunctionMock.returns(extensionResult);

    const result = validateStateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(extensionError);

    expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });

  it('should return valid result', () => {
    const extensionResult = new ValidationResult();

    extensionFunctionMock.returns(extensionResult);

    const result = validateStateTransitionStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });
});
