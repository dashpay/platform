const rewiremock = require('rewiremock/node');

const createAjv = require('../../../../../../../lib/ajv/createAjv');

const getAssetLockFixture = require('../../../../../../../lib/test/fixtures/getAssetLockFixture');
const JsonSchemaValidator = require('../../../../../../../lib/validation/JsonSchemaValidator');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const InvalidIdentityAssetLockProofError = require('../../../../../../../lib/errors/InvalidIdentityAssetLockProofError');
const IdentityAssetLockProofMismatchError = require('../../../../../../../lib/errors/IdentityAssetLockProofMismatchError');
const InvalidIdentityAssetLockProofSignatureError = require('../../../../../../../lib/errors/InvalidIdentityAssetLockProofSignatureError');

const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../../../../lib/test/expect/expectError',
);

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

describe('validateInstantAssetLockProofStructureFactory', () => {
  let rawProof;
  let transaction;
  let stateRepositoryMock;
  let InstantLockClassMock;
  let instantLockMock;
  let validateInstantAssetLockProofStructure;
  let smlStore;
  let jsonSchemaValidator;
  let validateInstantAssetLockProofStructureFactory;

  beforeEach(function beforeEach() {
    const assetLock = getAssetLockFixture();
    transaction = assetLock.getTransaction();

    rawProof = assetLock.getProof()
      .toObject();

    jsonSchemaValidator = new JsonSchemaValidator(createAjv());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    smlStore = {};

    stateRepositoryMock.fetchSMLStore.resolves(smlStore);

    instantLockMock = {
      txid: assetLock.getTransaction().id,
      verify: this.sinonSandbox.stub().resolves(true),
    };

    InstantLockClassMock = {
      fromBuffer: this.sinonSandbox.stub().returns(instantLockMock),
    };

    validateInstantAssetLockProofStructureFactory = rewiremock.proxy(
      '../../../../../../../lib/identity/stateTransitions/assetLock/proof/instant/validateInstantAssetLockProofStructureFactory',
      {
        '../../../../../../../node_modules/@dashevo/dashcore-lib': {
          InstantLock: InstantLockClassMock,
        },
      },
    );

    const skipAssetLockProofSignatureVerification = false;

    validateInstantAssetLockProofStructure = validateInstantAssetLockProofStructureFactory(
      jsonSchemaValidator,
      stateRepositoryMock,
      skipAssetLockProofSignatureVerification,
    );
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawProof.type;

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be equal to 0', async () => {
      rawProof.type = -1;

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('const');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });
  });

  describe('instantLock', () => {
    it('should be present', async () => {
      delete rawProof.instantLock;

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('instantLock');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be a byte array', async () => {
      rawProof.instantLock = new Array(165).fill('string');

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('.instantLock[0]');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be not shorter than 160 bytes', async () => {
      rawProof.instantLock = Buffer.alloc(159);

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.instantLock');
      expect(error.keyword).to.equal('minItems');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be not longer than 100 Kb', async () => {
      rawProof.instantLock = Buffer.alloc(100001);

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.instantLock');
      expect(error.keyword).to.equal('maxItems');

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should be valid', async () => {
      const instantLockError = new Error('something is wrong');

      InstantLockClassMock.fromBuffer.throws(instantLockError);

      rawProof.instantLock = Buffer.alloc(200);

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectValidationError(result, InvalidIdentityAssetLockProofError);

      const [error] = result.getErrors();

      expect(error.message).to.equal(`Invalid asset lock proof: ${instantLockError.message}`);

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should lock the same transaction', async () => {
      instantLockMock.txid = '123';

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectValidationError(result, IdentityAssetLockProofMismatchError);

      expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
      expect(instantLockMock.verify).to.not.be.called();
    });

    it('should have valid signature', async () => {
      instantLockMock.verify.resolves(false);

      const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

      expectValidationError(result, InvalidIdentityAssetLockProofSignatureError);

      expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnceWithExactly();
      expect(instantLockMock.verify).to.be.calledOnceWithExactly(smlStore);
    });
  });

  it('should skip signature verification if skipAssetLockProofSignatureVerification passed', async () => {
    const skipAssetLockProofSignatureVerification = true;

    validateInstantAssetLockProofStructure = validateInstantAssetLockProofStructureFactory(
      jsonSchemaValidator,
      stateRepositoryMock,
      skipAssetLockProofSignatureVerification,
    );

    const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchSMLStore).to.not.be.called();
    expect(instantLockMock.verify).to.not.be.called();
  });

  it('should return valid result', async () => {
    const result = await validateInstantAssetLockProofStructure(rawProof, transaction);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchSMLStore).to.be.calledOnceWithExactly();
    expect(instantLockMock.verify).to.be.calledOnceWithExactly(smlStore);
  });
});
