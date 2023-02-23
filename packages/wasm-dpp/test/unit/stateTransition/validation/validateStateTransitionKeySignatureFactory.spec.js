const { Transaction, PrivateKey, Script } = require('@dashevo/dashcore-lib');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../dist');

describe('validateStateTransitionKeySignatureFactory', () => {
  let stateTransition;
  let validateStateTransitionKeySignature;

  let InvalidStateTransitionSignatureError;
  let StateTransitionKeySignatureValidator;
  // let StateTransitionExecutionContext;
  let IdentityCreateTransition;
  let ValidationResult;

  before(async () => {
    ({
      InvalidStateTransitionSignatureError,
      StateTransitionKeySignatureValidator,
      // StateTransitionExecutionContext,
      IdentityCreateTransition,
      ValidationResult,
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    const stateTransitionJS = getIdentityCreateTransitionFixture();
    const rawStateTransition = stateTransitionJS.toObject();

    stateTransition = new IdentityCreateTransition(rawStateTransition);

    // const executionContext = new StateTransitionExecutionContext();
    // stateTransition.setExecutionContext(executionContext);

    const stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    const validator = new StateTransitionKeySignatureValidator(stateRepositoryMock);

    validateStateTransitionKeySignature = (st) => validator.validate(st);
  });

  it('should return invalid result if signature is not valid', async () => {
    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    await expectValidationError(result, InvalidStateTransitionSignatureError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(2002);
  });

  it('should return valid result if signature is valid', async () => {
    const rawStateTransition = stateTransition.toObject();

    // Sign state transition and provide relevant public key to transaction output
    const { transaction: rawTransaction } = rawStateTransition.assetLockProof;

    const transaction = new Transaction(Buffer.from(rawTransaction));

    const privateKey = new PrivateKey('9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2');
    const publicKey = privateKey.toPublicKey();

    transaction.outputs[0]
      .setScript(Script.buildDataOut(publicKey.hash));

    rawStateTransition.assetLockProof.transaction = transaction.toBuffer();

    stateTransition = new IdentityCreateTransition(rawStateTransition);

    await stateTransition.signByPrivateKey(
      privateKey.toBuffer(),
      0,
    );

    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    expect(result).to.be.instanceof(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
