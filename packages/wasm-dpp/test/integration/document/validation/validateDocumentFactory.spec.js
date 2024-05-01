const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const { expectJsonSchemaError, expectValidationError } = require('../../../../lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../dist');

let DocumentValidator;
let ProtocolVersionValidator;
let DataContract;
let InvalidDocumentTypeError;

describe.skip('validateDocumentFactory', () => {
  let dataContract;
  let dataContractJs;
  let rawDocuments;
  let rawDocument;
  let documentValidator;
  let ValidationResult;

  // eslint-disable-next-line prefer-arrow-callback
  beforeEach(async function beforeEach() {
    ({
      DocumentValidator,
      ProtocolVersionValidator,
      ValidationResult,
      DataContract,
      InvalidDocumentTypeError,
    } = await loadWasmDpp());

    dataContractJs = getDataContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    const protocolValidator = new ProtocolVersionValidator();
    documentValidator = new DocumentValidator(protocolValidator);

    rawDocuments = getDocumentsFixture(dataContractJs).map((o) => o.toObject());
    [rawDocument] = rawDocuments;
  });

  describe('Base schema', () => {
    describe('$protocolVersion', () => {
      it('should be present - Rust', async () => {
        delete rawDocument.$protocolVersion;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$protocolVersion');
      });

      it('should be an integer - Rust', async () => {
        rawDocument.$protocolVersion = '1';

        const result = documentValidator.validate(rawDocument, dataContract);
        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$protocolVersion');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be valid - Rust', async () => {
        rawDocument.$protocolVersion = -1;

        try {
          documentValidator.validate(rawDocument, dataContract);
        } catch (e) {
          // TODO - fix error when conversion errors are enabled
          expect(e.getMessage()).to.equal('integer out of bounds');
        }
      });
    });

    describe('$id', () => {
      it('should be present - Rust', async () => {
        delete rawDocument.$id;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$id');
      });

      it('should be a byte array - Rust', async () => {
        rawDocument.$id = new Array(32).fill('string');

        const result = documentValidator.validate(rawDocument, dataContract);
        // The jsonschema-rs behaves differently compared to the JS version.
        // It returns 32 errors and each is about the the type of the item in the array
        await expectJsonSchemaError(result, 32);
        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id/0');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be no less than 32 bytes - Rust', async () => {
        rawDocument.$id = Buffer.alloc(31);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes -  Rust', async () => {
        rawDocument.$id = Buffer.alloc(33);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$type', () => {
      let DataContractMock;

      afterEach(() => {
        if (DataContractMock) {
          DataContractMock.restore();
        }
      });

      it('should be present - Rust', async () => {
        delete rawDocument.$type;

        try {
          documentValidator.validate(rawDocument, dataContract);
        } catch (e) {
          // TODO - fix error when conversion errors are enabled
          // expect(error.getCode()).to.equal(1028);
          expect(e.getMessage()).to.be.equal('structure error: $type not found in map');
        }
      });

      it('should be defined in Data Contract - Rust', async () => {
        rawDocument.$type = 'undefinedDocument';

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectValidationError(
          result,
          InvalidDocumentTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1024);
        expect(error.getType()).to.equal('undefinedDocument');
      });

      it('should throw an error if getDocumentSchemaRef throws error - Rust', () => {
        // the test is impossible to trigger with the new wasm document validator
      });
    });

    describe('$revision', () => {
      it('should be present - Rust', async () => {
        delete rawDocument.$revision;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$revision');
      });

      it('should be a number - Rust', async () => {
        rawDocument.$revision = 'string';

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be an integer - Rust', async () => {
        rawDocument.$revision = 1.1;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be greater or equal to one - Rust', async () => {
        rawDocument.$revision = -1;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('minimum');
      });
    });

    describe('$dataContractId', () => {
      it('should be present - Rust', async () => {
        delete rawDocument.$dataContractId;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$dataContractId');
      });

      it('should be a byte array - Rust', async () => {
        rawDocument.$dataContractId = new Array(32).fill('string');

        const result = documentValidator.validate(rawDocument, dataContract);

        // The jsonschema-rs behaves differently compared to the JS version.
        // It returns 32 errors and each is about the the type of the item in the array
        await expectJsonSchemaError(result, 32);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId/0');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be no less than 32 bytes - Rust', async () => {
        rawDocument.$dataContractId = Buffer.alloc(31);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes - Rust', async () => {
        rawDocument.$dataContractId = Buffer.alloc(33);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$ownerId', () => {
      it('should be present - Rust', async () => {
        delete rawDocument.$ownerId;

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$ownerId');
      });

      it('should be a byte array - Rust', async () => {
        rawDocument.$ownerId = new Array(32).fill('string');

        const result = documentValidator.validate(rawDocument, dataContract);

        // The jsonschema-rs behaves differently compared to the JS version.
        // It returns 32 errors and each is about the the type of the item in the array
        await expectJsonSchemaError(result, 32);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$ownerId/0');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be no less than 32 bytes - Rust', async () => {
        rawDocument.$ownerId = Buffer.alloc(31);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$ownerId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes - Rust', async () => {
        rawDocument.$ownerId = Buffer.alloc(33);

        const result = documentValidator.validate(rawDocument, dataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$ownerId');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });
  });

  describe('Data Contract schema', () => {
    it('should return an error if the first document is not valid against Data Contract - Rust', async () => {
      rawDocument.name = 1;

      const result = documentValidator.validate(rawDocument, dataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/name');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should return an error if the second document is not valid against Data Contract - Rust', async () => {
      // eslint-disable-next-line prefer-destructuring
      rawDocument = rawDocuments[1];
      rawDocument.undefined = 1;

      const result = documentValidator.validate(rawDocument, dataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('additionalProperties');
    });
  });

  it('return invalid result if a byte array exceeds `maxItems` - Rust', async () => {
    // eslint-disable-next-line prefer-destructuring
    rawDocument = getDocumentsFixture(dataContractJs)[8].toObject();

    rawDocument.byteArrayField = Buffer.alloc(32);

    const result = documentValidator.validate(rawDocument, dataContract);

    await expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.getInstancePath()).to.equal('/byteArrayField');
    expect(error.getKeyword()).to.equal('maxItems');
  });

  it('should return valid result is a document is valid - Rust', async () => {
    const result = documentValidator.validate(rawDocument, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
