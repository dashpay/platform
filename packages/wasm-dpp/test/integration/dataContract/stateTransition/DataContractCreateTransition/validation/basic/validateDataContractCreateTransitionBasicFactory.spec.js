const dataContractMetaSchema = require('@dashevo/dpp/schema/dataContract/dataContractMeta.json');

const crypto = require('crypto');

// const { getRE2Class } = require('@dashevo/wasm-re2');

// const createAjv = require('@dashevo/dpp/lib/ajv/createAjv');

//const JsonSchemaValidator = require('@dashevo/dpp/lib/validation/JsonSchemaValidator');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { expectJsonSchemaError, expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const validateDataContractCreateTransitionBasicFactory = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/validation/basic/validateDataContractCreateTransitionBasicFactory');

//const DataContractCreateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

// const {
//   expectValidationError,
//   expectJsonSchemaError,
// } = require('@dashevo/dpp/lib/test/expect/expectError');

// const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

// const InvalidDataContractIdError = require('@dashevo/dpp/lib/errors/consensus/basic/dataContract/InvalidDataContractIdError');
// const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateDataContractCreateTransitionBasicFactory', () => {
  // let validateDataContractMock;
  let stateTransition;
  let rawStateTransition;
  let dataContract;
  let rawDataContract;
  // let validateProtocolVersionMock;

  let DataContractCreateTransition;
  let ProtocolVersionValidator;
  let validateDataContractCreateTransitionBasic;
  let ValidationResult;
  let ProtocolVersionParsingError;
  let InvalidDataContractIdError;

  before(async () => {
    ({
      DataContractCreateTransition,
      validateDataContractCreateTransitionBasic,
      ValidationResult,
      ProtocolVersionParsingError,
      InvalidDataContractIdError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    // validateDataContractMock = this.sinonSandbox.stub().returns(new ValidationResult());
    // validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResult());

    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });

    rawStateTransition = stateTransition.toObject();

    // const RE2 = await getRE2Class();
    // const ajv = createAjv(RE2);
    // const protocolVersionValidator = new ProtocolVersionValidator();
    // const jsonSchemaValidator = new JsonSchemaValidator(dataContractMetaSchema);

    // eslint-disable-next-line max-len
    // validateDataContractCreateTransitionBasic = validateDataContractCreateTransitionBasicFactory(
    //   jsonSchemaValidator,
    //   validateDataContractMock,
    //   validateProtocolVersionMock,
    // );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawStateTransition.protocolVersion = -1;

      // const protocolVersionError = new SomeConsensusError('test');
      // const protocolVersionResult = new ValidationResult([
      //   protocolVersionError,
      // ]);

      // validateProtocolVersionMock.returns(protocolVersionResult);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      // expectValidationError(result, SomeConsensusError);

      const [error] = result.getErrors();
      expect(error).to.be.an.instanceOf(ProtocolVersionParsingError);
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('type');
    });

    it('should be equal to 0', async () => {
      rawStateTransition.type = 666;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('const');
      expect(error.getParams().allowedValue).to.equal(0);
    });
  });

  describe('dataContract', () => {
    it('should be present', async () => {
      delete rawStateTransition.dataContract;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('dataContract');
    });

    it('should be valid', async () => {
      // const dataContractError = new SomeConsensusError('test');
      // const dataContractResult = new ValidationResult([
      //   dataContractError,
      // ]);

      // validateDataContractMock.returns(dataContractResult);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);
      
      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      // await expectValidationError(result);

      // const [error] = result.getErrors();

      // console.log(error.getCode());

      // expect(error).to.equal(dataContractError);

      // expect(validateDataContractMock.getCall(0).args).to.have.deep.members([rawDataContract]);
    });

    it('should return invalid result on invalid Data Contract id', async () => {
      // const dataContractResult = new ValidationResult();

      // validateDataContractMock.returns(dataContractResult);

      const expectedId = Buffer.from(rawStateTransition.dataContract.$id);
      rawStateTransition.dataContract.$id = Buffer.from(crypto.randomBytes(32));

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectValidationError(result);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1011);
      expect(error.getExpectedId()).to.deep.equal(expectedId);
      expect(error.getInvalidId()).to.deep.equal(rawStateTransition.dataContract.$id);
      expect(error).to.be.an.instanceOf(InvalidDataContractIdError);

    });
  });

  describe('entropy', () => {
    it('should be present', async () => {
      delete rawStateTransition.entropy;

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('entropy');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(31);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/entropy');
      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getParams().minItems).to.equal(32);
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.entropy = Buffer.alloc(33);

      const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/entropy');
      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getParams().maxItems).to.equal(32);
    });
  });

  // describe('signature', () => {
  //   it('should be present', async () => {
  //     delete rawStateTransition.signature;

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result);

  //     const [error] = result.getErrors();

  //     expect(error.getInstancePath()).to.equal('');
  //     expect(error.getKeyword()).to.equal('required');
  //     expect(error.getParams().missingProperty).to.equal('signature');
  //   });

  //   it('should be a byte array', async () => {
  //     rawStateTransition.signature = new Array(65).fill('string');

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result, 2);

  //     const [error, byteArrayError] = result.getErrors();

  //     expect(error.getInstancePath()).to.equal('/signature/0');
  //     expect(error.getKeyword()).to.equal('type');

  //     expect(byteArrayError.getKeyword()).to.equal('byteArray');
  //   });

  //   it('should be not less than 65 bytes', async () => {
  //     rawStateTransition.signature = Buffer.alloc(64);

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result);

  //     const [error] = result.getErrors();

  //     expect(error.getInstancePath()).to.equal('/signature');
  //     expect(error.getKeyword()).to.equal('minItems');
  //     expect(error.getParams().limit).to.equal(65);
  //   });

  //   it('should be not longer than 96 bytes', async () => {
  //     rawStateTransition.signature = Buffer.alloc(97);

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result);

  //     const [error] = result.getErrors();

  //     expect(error.getInstancePath()).to.equal('/signature');
  //     expect(error.getKeyword()).to.equal('maxItems');
  //     expect(error.getParams().limit).to.equal(96);
  //   });
  // });

  // describe('signaturePublicKeyId', () => {
  //   it('should be an integer', async () => {
  //     rawStateTransition.signaturePublicKeyId = 1.4;

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result, 1);

  //     const [error] = result.getErrors();

  //     expect(error.instancePath).to.equal('/signaturePublicKeyId');
  //     expect(error.getKeyword()).to.equal('type');
  //   });

  //   it('should not be < 0', async () => {
  //     rawStateTransition.signaturePublicKeyId = -1;

  //     const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

  //     expectJsonSchemaError(result, 1);

  //     const [error] = result.getErrors();

  //     expect(error.instancePath).to.equal('/signaturePublicKeyId');
  //     expect(error.getKeyword()).to.equal('minimum');
  //   });
  // });

  it('should return valid result', async () => {
    const result = await validateDataContractCreateTransitionBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
