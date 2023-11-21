const { PrivateKey, crypto: { Hash } } = require('@dashevo/dashcore-lib');

const crypto = require('crypto');

const { expect } = require('chai');
const BlsSignatures = require('../../../../lib/bls/bls');

const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const getBlsAdapterMock = require('../../../../lib/test/mocks/getBlsAdapterMock');

const { default: loadWasmDpp } = require('../../../../dist');

describe.skip('validatePublicKeySignaturesFactory', () => {
  let identityCreateTransition;
  let rawIdentityCreateTransition;
  let validatePublicKeySignatures;

  let IdentityPublicKey;
  let IdentityPublicKeyWithWitness;
  let InvalidIdentityKeySignatureError;
  let ValidationResult;
  let PublicKeysSignaturesValidator;
  let blsAdapter;

  before(async () => {
    ({
      IdentityPublicKey,
      IdentityPublicKeyWithWitness,
      InvalidIdentityKeySignatureError,
      PublicKeysSignaturesValidator,
      ValidationResult,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    identityCreateTransition = await getIdentityCreateTransitionFixture();

    const privateKey1 = new PrivateKey('17e0b1703e226204c557bce68b0871683ea409ae90c7a733b72a33f7c129c959');
    const publicKey1 = privateKey1.toPublicKey();

    const identityPublicKey1 = new IdentityPublicKeyWithWitness({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: publicKey1.toBuffer(),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
      signature: Buffer.alloc(0),
    });

    const privateKey2 = new PrivateKey('afc20afac882f676af5a268a2eca9c763996c36dbeb3660648df2108006820c7');
    const publicKey2 = privateKey2.toPublicKey();

    const identityPublicKey2 = new IdentityPublicKeyWithWitness({
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_HASH160,
      data: Hash.sha256ripemd160(publicKey2.toBuffer()),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
      readOnly: false,
      signature: Buffer.alloc(0),
    });

    const blsModule = await BlsSignatures.getInstance();
    blsAdapter = await getBlsAdapterMock();

    const { BasicSchemeMPL } = blsModule;

    const randomBytes = new Uint8Array(crypto.randomBytes(256));
    const privateKey3 = BasicSchemeMPL.keyGen(randomBytes);
    const publicKey3 = privateKey3.getG1();

    const identityPublicKey3 = new IdentityPublicKeyWithWitness({
      id: 2,
      type: IdentityPublicKey.TYPES.BLS12_381,
      data: Buffer.from(publicKey3.serialize()),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
      readOnly: false,
      signature: Buffer.alloc(0),
    });

    identityCreateTransition.setPublicKeys([
      identityPublicKey1,
      identityPublicKey2,
      identityPublicKey3,
    ]);

    await identityCreateTransition.signByPrivateKey(
      privateKey1.toBuffer(),
      IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    );

    const signature1 = identityCreateTransition.getSignature();

    await identityCreateTransition.signByPrivateKey(
      privateKey2.toBuffer(),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    const signature2 = identityCreateTransition.getSignature();

    await identityCreateTransition.signByPrivateKey(
      Buffer.from(privateKey3.serialize()),
      IdentityPublicKey.TYPES.BLS12_381,
      blsAdapter,
    );

    const signature3 = identityCreateTransition.getSignature();

    identityPublicKey1.setSignature(signature1);
    identityPublicKey2.setSignature(signature2);
    identityPublicKey3.setSignature(signature3);

    identityCreateTransition.setPublicKeys([
      identityPublicKey1,
      identityPublicKey2,
      identityPublicKey3,
    ]);

    rawIdentityCreateTransition = identityCreateTransition.toObject();

    const validator = new PublicKeysSignaturesValidator(blsAdapter);
    validatePublicKeySignatures = (stateTransition, keys) => validator.validate(
      stateTransition,
      keys,
    );

    privateKey3.delete();
    publicKey3.delete();
  });

  it('should return InvalidIdentityKeySignatureError if signature is not valid', async () => {
    const rawPublicKey2 = rawIdentityCreateTransition.publicKeys[1];

    rawPublicKey2.signature = crypto.randomBytes(65);

    const result = await validatePublicKeySignatures(
      rawIdentityCreateTransition,
      rawIdentityCreateTransition.publicKeys,
    );

    await expectValidationError(result, InvalidIdentityKeySignatureError);

    const error = result.getFirstError();

    expect(error.getPublicKeyId()).to.equals(rawPublicKey2.id);
  });

  it('should return valid result', async () => {
    const result = await validatePublicKeySignatures(
      rawIdentityCreateTransition,
      rawIdentityCreateTransition.publicKeys,
      blsAdapter,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
