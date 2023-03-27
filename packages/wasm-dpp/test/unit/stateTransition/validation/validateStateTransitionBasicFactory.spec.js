const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getPrivateAndPublicKeyFixture = require('../../../../lib/test/fixtures/getPrivateAndPublicKeyForSigningFixture');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../..');
let {
  DashPlatformProtocol,
  MissingStateTransitionTypeError,
  InvalidStateTransitionTypeError,
  StateTransitionMaxSizeExceededError,
  JsonSchemaError,
  ValidationResult,
} = require('../../../..');

describe('validateStateTransitionBasicFactory', () => {
  let rawStateTransition;
  let dataContract;
  let stateTransition;
  let dpp;

  beforeEach(async function beforeEach() {
    ({
      DashPlatformProtocol,
      MissingStateTransitionTypeError,
      InvalidStateTransitionTypeError,
      StateTransitionMaxSizeExceededError,
      JsonSchemaError,
      ValidationResult,
    } = await loadWasmDpp());

    const stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    const blsMock = getBlsMock();

    dpp = new DashPlatformProtocol(stateRepositoryMock, blsMock);

    dataContract = await getDataContractFixture();

    const { privateKey, identityPublicKey } = await getPrivateAndPublicKeyFixture();

    stateTransition = await dpp.dataContract.createDataContractCreateTransition(dataContract);

    await stateTransition.sign(identityPublicKey, privateKey, getBlsMock());

    rawStateTransition = stateTransition.toObject();
  });

  it('should return invalid result if ST type is missing', async () => {
    delete rawStateTransition.type;

    const result = await dpp.stateTransition.validateBasic(rawStateTransition);

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceof(MissingStateTransitionTypeError);
  });

  it('should return invalid result if ST type is not valid', async () => {
    rawStateTransition.type = 66;

    const result = await dpp.stateTransition.validateBasic(rawStateTransition);

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceof(InvalidStateTransitionTypeError);
  });

  it('should return invalid result if ST is invalid against validation function', async () => {
    delete rawStateTransition.signaturePublicKeyId;
    const result = await dpp.stateTransition.validateBasic(rawStateTransition);

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.instanceof(JsonSchemaError);
  });

  it('should return invalid result if ST size is more than 16 kb', async () => {
    const largeDocument = rawStateTransition.dataContract.documents.niceDocument;

    // generate big state transition
    for (let i = 0; i < 6; i++) {
      largeDocument.properties[`name${i}`] = { type: 'string' };
    }
    for (let i = 0; i < 90; i++) {
      rawStateTransition.dataContract.documents[`anotherContract${i}`] = largeDocument;
    }

    const result = await dpp.stateTransition.validateBasic(
      rawStateTransition,
    );

    await expectValidationError(result, StateTransitionMaxSizeExceededError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1045);
  });

  it('should return valid result', async () => {
    const result = await dpp.stateTransition.validateBasic(rawStateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
