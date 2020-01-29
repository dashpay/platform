const Ajv = require('ajv');

const validateStateTransitionStructureFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionStructureFactory');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');

const DocumentsStateTransition = require('../../../../lib/document/stateTransition/DocumentsStateTransition');
const DataContractStateTransition = require('../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const dataContractSTSchema = require('../../../../schema/stateTransition/data-contract');
const documentsSTSchema = require('../../../../schema/stateTransition/documents');
const identitySTSchema = require('../../../../schema/identity/state-transitions/identity-create');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getIdentityCreateSTFixture = require('../../../../lib/test/fixtures/getIdentityCreateSTFixture');

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
  let validator;
  let extensionFunctionMock;
  let rawStateTransition;
  let dataContract;
  let privateKey;

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
        validationFunction: extensionFunctionMock,
        schema: extensionSchema,
      },
    };

    const ajv = new Ajv();
    validator = new JsonSchemaValidator(ajv);

    validateStateTransitionStructure = validateStateTransitionStructureFactory(
      validator,
      typeExtensions,
    );

    dataContract = getDataContractFixture();

    privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    rawStateTransition = {
      protocolVersion: 0,
      type: stateTransitionTypes.DATA_CONTRACT,
      extension: {},
      signature: 'H8tcxA468bMRB5183MER6xud6olAXfxutwDQiv5vaiN8AXFkup6jkSXWQdmaVF5Wvw2ppkYxXAGsBI2N94OMxvw=',
    };
  });

  describe('Base schema', () => {
    describe('protocolVersion', () => {
      it('should be present', async () => {
        delete rawStateTransition.protocolVersion;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('protocolVersion');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should equal to 0', async () => {
        rawStateTransition.protocolVersion = 666;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.protocolVersion');
        expect(error.keyword).to.equal('const');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('type', () => {
      it('should be present', async () => {
        delete rawStateTransition.type;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectValidationError(
          result,
          MissingStateTransitionTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawStateTransition()).to.equal(rawStateTransition);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have defined extension', async () => {
        rawStateTransition.type = 666;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectValidationError(
          result,
          InvalidStateTransitionTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawStateTransition()).to.equal(rawStateTransition);

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('signature', () => {
      it('should be present', async () => {
        delete rawStateTransition.signature;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('signature');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should no have length < 86', async () => {
        rawStateTransition.signature = 'xqA468bMRB5183MER6xud6olAXfxutwDQiv5vaiN8AXFkup6jkSXWQdmaVF5Wvw2ppkYxXAGsBI2N94OMxvw=';

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.signature');
        expect(error.keyword).to.equal('minLength');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not have length > 88', async () => {
        rawStateTransition.signature = 'H8tcxqwertyA468bMRB5183MER6xud6olAXfxutwDQiv5vaiN8AXFkup6jkSXWQdmaVF5Wvw2ppkYxXAGsBI2N94OMxvw=';

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.signature');
        expect(error.keyword).to.equal('maxLength');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be base64 encoded', async () => {
        rawStateTransition.signature = '&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&';

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.signature');
        expect(error.keyword).to.equal('pattern');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('signaturePublicKeyId', () => {
      it('should be an integer', async () => {
        rawStateTransition.signaturePublicKeyId = 1.4;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result, 1);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.signaturePublicKeyId');
        expect(error.keyword).to.equal('type');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be a nullable', async () => {
        const extensionResult = new ValidationResult();

        extensionFunctionMock.returns(extensionResult);

        rawStateTransition.signaturePublicKeyId = null;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();

        expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
      });

      it('should not be < 1', async () => {
        rawStateTransition.signaturePublicKeyId = 0;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result, 1);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.signaturePublicKeyId');
        expect(error.keyword).to.equal('minimum');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });
  });

  describe('Data Contract Schema', () => {
    beforeEach(() => {
      const typeExtensions = {
        [stateTransitionTypes.DATA_CONTRACT]: {
          validationFunction: extensionFunctionMock,
          schema: dataContractSTSchema,
        },
      };

      validateStateTransitionStructure = validateStateTransitionStructureFactory(
        validator,
        typeExtensions,
      );

      const stateTransition = new DataContractStateTransition(dataContract);
      stateTransition.signByPrivateKey(privateKey);

      rawStateTransition = stateTransition.toJSON();
    });

    describe('dataContract', () => {
      it('should be present', async () => {
        delete rawStateTransition.dataContract;

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('dataContract');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    it('should be valid', async () => {
      extensionFunctionMock.returns(new ValidationResult());

      const result = await validateStateTransitionStructure(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
      expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
    });
  });

  describe('Documents Schema', () => {
    beforeEach(() => {
      const typeExtensions = {
        [stateTransitionTypes.DOCUMENTS]: {
          validationFunction: extensionFunctionMock,
          schema: documentsSTSchema,
        },
      };

      validateStateTransitionStructure = validateStateTransitionStructureFactory(
        validator,
        typeExtensions,
      );

      const documents = getDocumentsFixture();

      const stateTransition = new DocumentsStateTransition(documents);
      stateTransition.signByPrivateKey(privateKey);

      rawStateTransition = stateTransition.toJSON();
    });

    describe('actions', () => {
      it('should be present', async () => {
        delete rawStateTransition.actions;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('actions');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be an array', async () => {
        rawStateTransition.actions = {};

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.actions');
        expect(error.keyword).to.equal('type');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have at least one element', async () => {
        rawStateTransition.actions = [];

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.actions');
        expect(error.keyword).to.equal('minItems');
        expect(error.params.limit).to.equal(1);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have no more than 10 elements', async () => {
        rawStateTransition.actions = Array(11).fill({});

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.actions');
        expect(error.keyword).to.equal('maxItems');
        expect(error.params.limit).to.equal(10);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have action types as elements', async () => {
        rawStateTransition.actions = [{}];

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.actions[0]');
        expect(error.keyword).to.equal('type');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('documents', () => {
      it('should be present', async () => {
        delete rawStateTransition.documents;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('documents');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be an array', async () => {
        rawStateTransition.documents = {};

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents');
        expect(error.keyword).to.equal('type');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have at least one element', async () => {
        rawStateTransition.documents = [];

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents');
        expect(error.keyword).to.equal('minItems');
        expect(error.params.limit).to.equal(1);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have no more than 10 elements', async () => {
        rawStateTransition.documents = Array(11).fill({});

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents');
        expect(error.keyword).to.equal('maxItems');
        expect(error.params.limit).to.equal(10);

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should have objects as elements', async () => {
        rawStateTransition.documents = [1];

        const result = await validateStateTransitionStructure(rawStateTransition);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[0]');
        expect(error.keyword).to.equal('type');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    it('should be valid', async () => {
      extensionFunctionMock.returns(new ValidationResult());

      const result = await validateStateTransitionStructure(
        rawStateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
    });
  });

  describe('Identity schema', () => {
    beforeEach(() => {
      const typeExtensions = {
        [stateTransitionTypes.IDENTITY_CREATE]: {
          validationFunction: extensionFunctionMock,
          schema: identitySTSchema,
        },
      };

      validateStateTransitionStructure = validateStateTransitionStructureFactory(
        validator,
        typeExtensions,
      );

      const stateTransition = getIdentityCreateSTFixture();
      stateTransition.signByPrivateKey(privateKey);

      rawStateTransition = stateTransition.toJSON();
    });

    describe('lockedOutPoint', () => {
      it('should be present', async () => {
        rawStateTransition.lockedOutPoint = undefined;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.params.missingProperty).to.equal('lockedOutPoint');
        expect(error.keyword).to.equal('required');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not be less than 48 characters in length', async () => {
        rawStateTransition.lockedOutPoint = '1';

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('minLength');
        expect(error.dataPath).to.equal('.lockedOutPoint');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not be more than 48 characters in length', async () => {
        rawStateTransition.lockedOutPoint = Buffer.alloc(48).toString('base64');

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('maxLength');
        expect(error.dataPath).to.equal('.lockedOutPoint');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be base64 encoded', async () => {
        rawStateTransition.lockedOutPoint = '&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&';

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('pattern');
        expect(error.dataPath).to.equal('.lockedOutPoint');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('identityType', () => {
      it('should be present', async () => {
        rawStateTransition.identityType = undefined;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.params.missingProperty).to.equal('identityType');
        expect(error.keyword).to.equal('required');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should be an integer', async () => {
        rawStateTransition.identityType = 1.2;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );


        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('multipleOf');
        expect(error.dataPath).to.equal('.identityType');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not be less than 0', async () => {
        rawStateTransition.identityType = -1;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('minimum');
        expect(error.dataPath).to.equal('.identityType');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not be more than 65535', async () => {
        rawStateTransition.identityType = 100000;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('maximum');
        expect(error.dataPath).to.equal('.identityType');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    describe('publicKeys', () => {
      it('should be present', async () => {
        rawStateTransition.publicKeys = undefined;

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.params.missingProperty).to.equal('publicKeys');
        expect(error.keyword).to.equal('required');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not be empty', async () => {
        rawStateTransition.publicKeys = [];

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('minItems');
        expect(error.dataPath).to.equal('.publicKeys');

        expect(extensionFunctionMock).to.not.be.called();
      });

      it('should not have more than 10 items', async () => {
        const [key] = rawStateTransition.publicKeys;

        for (let i = 0; i < 10; i++) {
          rawStateTransition.publicKeys.push(key);
        }

        const result = await validateStateTransitionStructure(
          rawStateTransition,
        );

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.keyword).to.equal('maxItems');
        expect(error.dataPath).to.equal('.publicKeys');

        expect(extensionFunctionMock).to.not.be.called();
      });
    });

    it('should be valid', async () => {
      extensionFunctionMock.returns(new ValidationResult());

      const result = await validateStateTransitionStructure(
        rawStateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
    });
  });

  it('should return invalid result if ST invalid against extension schema', async () => {
    delete rawStateTransition.extension;

    const result = await validateStateTransitionStructure(rawStateTransition);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('required');
    expect(error.params.missingProperty).to.equal('extension');

    expect(extensionFunctionMock).to.not.be.called();
  });

  it('should return invalid result if ST is invalid against extension function', async () => {
    const extensionError = new ConsensusError('test');
    const extensionResult = new ValidationResult([
      extensionError,
    ]);

    extensionFunctionMock.returns(extensionResult);

    const result = await validateStateTransitionStructure(rawStateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(extensionError);

    expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });

  it('should return valid result', async () => {
    const extensionResult = new ValidationResult();

    extensionFunctionMock.returns(extensionResult);

    const result = await validateStateTransitionStructure(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(extensionFunctionMock).to.be.calledOnceWith(rawStateTransition);
  });
});
