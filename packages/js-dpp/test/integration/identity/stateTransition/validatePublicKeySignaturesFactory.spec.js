const { PrivateKey, crypto: { Hash } } = require('@dashevo/dashcore-lib');

const crypto = require('crypto');

const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const IdentityPublicKey = require('../../../../lib/identity/IdentityPublicKey');
const BlsSignatures = require('../../../../lib/bls/bls');
const validatePublicKeySignaturesFactory = require('../../../../lib/identity/stateTransition/validatePublicKeySignaturesFactory');
const ValidationResult = require('../../../../lib/validation/ValidationResult');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const InvalidIdentityKeySignatureError = require('../../../../lib/errors/consensus/basic/identity/InvalidIdentityKeySignatureError');

describe('validatePublicKeySignaturesFactory', () => {
  let identityCreateTransition;
  let rawIdentityCreateTransition;
  let validatePublicKeySignatures;

  beforeEach(async function beforeEach() {
    identityCreateTransition = getIdentityCreateTransitionFixture();

    const privateKey1 = new PrivateKey();
    const publicKey1 = privateKey1.toPublicKey();

    const identityPublicKey1 = new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: publicKey1.toBuffer(),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    });

    const privateKey2 = new PrivateKey();
    const publicKey2 = privateKey2.toPublicKey();

    const identityPublicKey2 = new IdentityPublicKey({
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_HASH160,
      data: Hash.sha256ripemd160(publicKey2.toBuffer()),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
      readOnly: false,
    });

    const { PrivateKey: BlsPrivateKey } = await BlsSignatures.getInstance();

    const randomBytes = new Uint8Array(crypto.randomBytes(256));
    const privateKey3 = BlsPrivateKey.fromBytes(randomBytes, true);
    // blsPrivateKeyHex = Buffer.from(blsPrivateKey.serialize()).toString('hex');
    const publicKey3 = privateKey3.getPublicKey();

    const identityPublicKey3 = new IdentityPublicKey({
      id: 2,
      type: IdentityPublicKey.TYPES.BLS12_381,
      data: Buffer.from(publicKey3.serialize()),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
      readOnly: false,
    });

    identityCreateTransition.setPublicKeys([
      identityPublicKey1,
      identityPublicKey2,
      identityPublicKey3,
    ]);

    await identityCreateTransition.signByPrivateKey(
      privateKey1,
      IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    );

    const signature1 = identityCreateTransition.getSignature();

    await identityCreateTransition.signByPrivateKey(
      privateKey2,
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    const signature2 = identityCreateTransition.getSignature();

    await identityCreateTransition.signByPrivateKey(
      Buffer.from(privateKey3.serialize()),
      IdentityPublicKey.TYPES.BLS12_381,
    );

    const signature3 = identityCreateTransition.getSignature();

    identityPublicKey1.setSignature(signature1);
    identityPublicKey2.setSignature(signature2);
    identityPublicKey3.setSignature(signature3);

    rawIdentityCreateTransition = identityCreateTransition.toObject();

    const createStateTransitionMock = this.sinonSandbox.stub().resolves(identityCreateTransition);

    validatePublicKeySignatures = validatePublicKeySignaturesFactory(createStateTransitionMock);
  });

  it('should return InvalidIdentityKeySignatureError if signature is not valid', async () => {
    const rawPublicKey2 = rawIdentityCreateTransition.publicKeys[1];

    rawPublicKey2.signature = crypto.randomBytes(65);

    const result = await validatePublicKeySignatures(
      rawIdentityCreateTransition,
      rawIdentityCreateTransition.publicKeys,
      identityCreateTransition.getExecutionContext(),
    );

    expectValidationError(result, InvalidIdentityKeySignatureError);

    const error = result.getFirstError();

    expect(error.getPublicKeyId()).to.equals(rawPublicKey2.id);
  });

  it('should return valid result', async () => {
    const result = await validatePublicKeySignatures(
      rawIdentityCreateTransition,
      rawIdentityCreateTransition.publicKeys,
      identityCreateTransition.getExecutionContext(),
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
