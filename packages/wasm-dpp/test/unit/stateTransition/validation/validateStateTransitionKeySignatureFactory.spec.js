const { Transaction, PrivateKey, Script } = require('@dashevo/dashcore-lib');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getIdentityTopUpTransitionFixture = require('../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../dist');

describe.skip('validateStateTransitionKeySignatureFactory', () => {
  let stateTransition;
  let stateRepositoryMock;
  let validateStateTransitionKeySignature;
  let executionContext;

  let InvalidStateTransitionSignatureError;
  let StateTransitionKeySignatureValidator;
  let StateTransitionExecutionContext;
  let IdentityCreateTransition;
  let IdentityNotFoundError;
  let ValidationResult;

  before(async () => {
    ({
      InvalidStateTransitionSignatureError,
      StateTransitionKeySignatureValidator,
      StateTransitionExecutionContext,
      IdentityCreateTransition,
      IdentityNotFoundError,
      ValidationResult,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    executionContext = new StateTransitionExecutionContext();
    stateTransition = await getIdentityCreateTransitionFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    const validator = new StateTransitionKeySignatureValidator(stateRepositoryMock);

    validateStateTransitionKeySignature = (st) => validator.validate(st, executionContext);
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

  it('should return IdentityNotFoundError if identity not exist on topup transaction', async function shouldReturn() {
    stateTransition = await getIdentityTopUpTransitionFixture();
    stateRepositoryMock.fetchIdentityBalance.resolves(undefined);

    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    await expectValidationError(result, IdentityNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(2000);

    const { match } = this.sinon;
    expect(stateRepositoryMock.fetchIdentityBalance).to.be.calledOnceWithExactly(
      match((identityId) => Buffer.from(identityId.toBuffer())
        .equals(stateTransition.getIdentityId().toBuffer())),
      match.instanceOf(StateTransitionExecutionContext),
    );
  });
});
