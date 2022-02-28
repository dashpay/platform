const { PrivateKey, crypto: { Hash } } = require('@dashevo/dashcore-lib');

const crypto = require('crypto');
const calculateStateTransitionFee = require('../../../lib/stateTransition/calculateStateTransitionFee');

const StateTransitionMock = require('../../../lib/test/mocks/StateTransitionMock');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const InvalidSignatureTypeError = require('../../../lib/stateTransition/errors/InvalidIdentityPublicKeyTypeError');
const InvalidSignaturePublicKeyError = require('../../../lib/stateTransition/errors/InvalidSignaturePublicKeyError');
const PublicKeySecurityLevelNotMetError = require('../../../lib/stateTransition/errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('../../../lib/stateTransition/errors/WrongPublicKeyPurposeError');
const StateTransitionIsNotSignedError = require('../../../lib/stateTransition/errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('../../../lib/stateTransition/errors/PublicKeyMismatchError');
const BlsSignatures = require('../../../lib/bls/bls');

describe('AbstractStateTransitionIdentitySigned', () => {
  let stateTransition;
  let protocolVersion;
  let privateKeyHex;
  let privateKeyWIF;
  let publicKeyId;
  let identityPublicKey;
  let blsPrivateKey;
  let blsPrivateKeyHex;
  let blsInstance;

  beforeEach(async () => {
    const privateKeyModel = new PrivateKey();
    privateKeyWIF = privateKeyModel.toWIF();
    privateKeyHex = privateKeyModel.toBuffer().toString('hex');
    const publicKey = privateKeyModel.toPublicKey().toBuffer();
    publicKeyId = 1;

    protocolVersion = 1;

    stateTransition = new StateTransitionMock({
      protocolVersion,
    });

    blsInstance = await BlsSignatures.getInstance();
    const {
      PrivateKey: BlsPrivateKey,
    } = blsInstance;

    const randomBytes = new Uint8Array(crypto.randomBytes(256));
    blsPrivateKey = BlsPrivateKey.fromBytes(randomBytes, true);
    blsPrivateKeyHex = Buffer.from(blsPrivateKey.serialize()).toString('hex');

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER)
      .setPurpose(IdentityPublicKey.PURPOSES.AUTHENTICATION);
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      const rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion,
        signature: undefined,
        signaturePublicKeyId: undefined,
        type: 0,
      });
    });

    it('should return raw state transition without signature ', () => {
      const rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion,
        type: 0,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return state transition as JSON', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        signaturePublicKeyId: undefined,
        signature: undefined,
        protocolVersion,
        type: 0,
      });
    });
  });

  describe('#hash', () => {
    it.skip('should return serialized hash', () => {
      const hash = stateTransition.hash();

      expect(hash).to.be.equal('9177fcb220cbab84abcb9ebd5c048facf47f455f6826bf37d97a9908e09fcafd');
    });
  });

  describe('#toBuffer', () => {
    it.skip('should return serialized data', () => {
      const serializedData = stateTransition.toBuffer();

      expect(serializedData.toString('hex')).to.be.equal('a4647479706500697369676e6174757265f66f70726f746f636f6c56657273696f6ef6747369676e61747572655075626c69634b65794964f6');
    });

    it.skip('should return serialized data without signature data', () => {
      const serializedData = stateTransition.toBuffer({ skipSignature: true });

      expect(serializedData.toString('hex')).to.be.equal('a26474797065006f70726f746f636f6c56657273696f6ef6');
    });
  });

  describe('#getSignaturePublicKeyId', () => {
    it('should return public key ID', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      const keyId = stateTransition.getSignaturePublicKeyId();
      expect(keyId).to.be.equal(publicKeyId);
    });
  });

  describe('#sign', () => {
    it('should sign data and validate signature with private key in hex format', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should sign data and validate signature with private key in buffer format', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyWIF);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should sign data and validate signature with ECDSA_HASH160 identityPublicKey', async () => {
      identityPublicKey.setType(IdentityPublicKey.TYPES.ECDSA_HASH160);
      identityPublicKey.setData(
        Hash.sha256ripemd160(identityPublicKey.getData()),
      );

      await stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an error if we try to sign with wrong public key', async () => {
      const publicKey = new PrivateKey()
        .toPublicKey()
        .toBuffer();

      identityPublicKey.setData(publicKey);

      try {
        await stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw InvalidSignaturePublicKeyError');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidSignaturePublicKeyError);
        expect(e.getSignaturePublicKey()).to.deep.equal(identityPublicKey.getData());
      }
    });

    it('should throw InvalidSignatureTypeError if signature type is not equal ECDSA', async () => {
      identityPublicKey.setType(30000);

      try {
        await stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw InvalidSignatureTypeError');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidSignatureTypeError);
        expect(e.getPublicKeyType()).to.be.equal(identityPublicKey.getType());
      }
    });

    it('should throw an error if the key security level is not met', async function () {
      stateTransition.getRequiredKeySecurityLevel = this.sinonSandbox
        .stub()
        .returns(IdentityPublicKey.SECURITY_LEVELS.MASTER);

      identityPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

      try {
        await stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw PublicKeySecurityLevelNotMetError');
      } catch (e) {
        expect(e).to.be.instanceOf(PublicKeySecurityLevelNotMetError);
        expect(e.getPublicKeySecurityLevel())
          .to.be.deep.equal(identityPublicKey.getSecurityLevel());
        expect(e.getKeySecurityLevelRequirement())
          .to.be.deep.equal(IdentityPublicKey.SECURITY_LEVELS.MASTER);
      }
    });

    it('should throw an error if the key purpose is not authentication', async () => {
      identityPublicKey.setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);

      try {
        await stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw WrongPublicKeyPurposeError');
      } catch (e) {
        expect(e).to.be.instanceOf(WrongPublicKeyPurposeError);
        expect(e.getPublicKeyPurpose()).to.be.deep.equal(identityPublicKey.getPurpose());
        expect(e.getKeyPurposeRequirement())
          .to.be.deep.equal(IdentityPublicKey.PURPOSES.AUTHENTICATION);
      }
    });

    it('should sign data and validate signature with BLS12_381 identityPublicKey', async () => {
      identityPublicKey.setType(IdentityPublicKey.TYPES.BLS12_381);
      identityPublicKey.setData(Buffer.from(blsPrivateKey.getPublicKey().serialize()));

      await stateTransition.sign(identityPublicKey, blsPrivateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });
  });

  describe('#signByPrivateKey', () => {
    it('should sign and validate with private key', async () => {
      privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

      await stateTransition.signByPrivateKey(
        privateKeyHex,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);
    });

    it('should sign and validate with BLS private key', async () => {
      identityPublicKey.setType(IdentityPublicKey.TYPES.BLS12_381);
      identityPublicKey.setData(Buffer.from(blsPrivateKey.getPublicKey().serialize()));

      await stateTransition.signByPrivateKey(blsPrivateKeyHex, IdentityPublicKey.TYPES.BLS12_381);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifyBLSSignatureByPublicKey(
        blsPrivateKey.getPublicKey(),
      );

      expect(isValid).to.be.true();
    });
  });

  describe('#verifySignature', () => {
    it('should validate signature', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', async () => {
      try {
        await stateTransition.verifySignature(identityPublicKey);

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });

    it('should throw an PublicKeyMismatchError error if public key id not equals public key id in state transition', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      identityPublicKey.setId(identityPublicKey.getId() + 1);

      try {
        await stateTransition.verifySignature(identityPublicKey);

        expect.fail('should throw PublicKeyMismatchError');
      } catch (e) {
        expect(e).to.be.instanceOf(PublicKeyMismatchError);
        expect(e.getPublicKey()).to.equal(identityPublicKey);
      }
    });

    it('should not verify signature with wrong public key', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);
      const publicKey = new PrivateKey()
        .toPublicKey()
        .toBuffer();

      identityPublicKey.setData(publicKey);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.false();
    });

    it('should throw an error if the key security level is not met', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      // Set key security level after the signing, since otherwise .sign method won't work
      identityPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

      try {
        await stateTransition.verifySignature(identityPublicKey);

        expect.fail('Should throw PublicKeySecurityLevelNotMetError');
      } catch (e) {
        expect(e).to.be.instanceOf(PublicKeySecurityLevelNotMetError);
        expect(e.getPublicKeySecurityLevel())
          .to.be.deep.equal(identityPublicKey.getSecurityLevel());
        expect(e.getKeySecurityLevelRequirement())
          .to.be.deep.equal(IdentityPublicKey.SECURITY_LEVELS.MASTER);
      }
    });

    it('should throw an error if the key purpose is not equal to authentication', async () => {
      await stateTransition.sign(identityPublicKey, privateKeyHex);

      // Set key security level after the signing, since otherwise .sign method won't work
      identityPublicKey.setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);

      try {
        await stateTransition.verifySignature(identityPublicKey);

        expect.fail('Should throw WrongPublicKeyPurposeError');
      } catch (e) {
        expect(e).to.be.instanceOf(WrongPublicKeyPurposeError);
        expect(e.getPublicKeyPurpose()).to.be.deep.equal(identityPublicKey.getPurpose());
        expect(e.getKeyPurposeRequirement())
          .to.be.deep.equal(IdentityPublicKey.PURPOSES.AUTHENTICATION);
      }
    });

    it('should validate BLS signature', async () => {
      identityPublicKey.setType(IdentityPublicKey.TYPES.BLS12_381);
      identityPublicKey.setData(Buffer.from(blsPrivateKey.getPublicKey().serialize()));

      await stateTransition.sign(identityPublicKey, blsPrivateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(Buffer);

      const isValid = await stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });
  });

  describe('#verifyESDSAHash160SignatureByPublicKeyHash', () => {
    it('should validate sign by public key hash', async () => {
      privateKeyHex = 'fdfa0d878967ac17ca3e6fa6ca7f647fea51cffac85e41424c6954fcbe97721c';
      const publicKey = 'dLfavDCp+ARA3O0AXsOFJ0W//mg=';

      await stateTransition.signByPrivateKey(privateKeyHex, IdentityPublicKey.TYPES.ECDSA_HASH160);

      const isValid = stateTransition.verifyESDSAHash160SignatureByPublicKeyHash(Buffer.from(publicKey, 'base64'));

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', async () => {
      const publicKey = 'dLfavDCp+ARA3O0AXsOFJ0W//mg=';
      try {
        stateTransition.verifyESDSAHash160SignatureByPublicKeyHash(Buffer.from(publicKey, 'base64'));

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });
  });

  describe('#verifyECDSASignatureByPublicKey', () => {
    it('should validate sign by public key', async () => {
      privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';
      const publicKey = 'A1eUrJ7lM6F1m6dbIyk+vXimKfzki+QRMHMwoAmggt6L';

      await stateTransition.signByPrivateKey(
        privateKeyHex,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      const isValid = stateTransition.verifyECDSASignatureByPublicKey(Buffer.from(publicKey, 'base64'));

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', async () => {
      const publicKey = 'A1eUrJ7lM6F1m6dbIyk+vXimKfzki+QRMHMwoAmggt6L';
      try {
        stateTransition.verifyECDSASignatureByPublicKey(Buffer.from(publicKey, 'base64'));

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });
  });

  describe('#verifyBLSSignatureByPublicKey', () => {
    it('should validate sign by public key', async () => {
      const publicKey = blsPrivateKey.getPublicKey();

      identityPublicKey.setType(IdentityPublicKey.TYPES.BLS12_381);
      identityPublicKey.setData(Buffer.from(publicKey.serialize()));

      await stateTransition.signByPrivateKey(blsPrivateKeyHex, IdentityPublicKey.TYPES.BLS12_381);

      const isValid = await stateTransition.verifyBLSSignatureByPublicKey(publicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', async () => {
      const publicKey = Buffer.from(blsPrivateKey.getPublicKey().serialize());
      try {
        await stateTransition.verifyBLSSignatureByPublicKey(publicKey);

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });
  });

  describe('#setSignature', () => {
    it('should set signature', () => {
      const signature = 'A1eUrA';
      stateTransition.setSignature(signature);

      expect(stateTransition.signature.toString()).to.equal(signature);
    });
  });

  describe('#setSignaturePublicKeyId', () => {
    it('should set signature public key id', async () => {
      const signaturePublicKeyId = 1;
      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      expect(stateTransition.signaturePublicKeyId).to.equal(signaturePublicKeyId);
    });
  });

  describe('#calculateFee', () => {
    it('should calculate fee', () => {
      const result = stateTransition.calculateFee();

      const fee = calculateStateTransitionFee(stateTransition);

      expect(result).to.equal(fee);
    });
  });
});
