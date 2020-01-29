const { PrivateKey } = require('@dashevo/dashcore-lib');

const StateTransitionMock = require('../../../lib/test/mocks/StateTransitionMock');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const InvalidSignatureTypeError = require('../../../lib/stateTransition/errors/InvalidSignatureTypeError');
const InvalidSignaturePublicKeyError = require('../../../lib/stateTransition/errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('../../../lib/stateTransition/errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('../../../lib/stateTransition/errors/PublicKeyMismatchError');

describe('AbstractStateTransition', () => {
  let stateTransition;
  let privateKeyHex;
  let privateKeyBuffer;
  let publicKeyId;
  let identityPublicKey;

  beforeEach(() => {
    const privateKeyModel = new PrivateKey();
    privateKeyBuffer = privateKeyModel.toBuffer();
    privateKeyHex = privateKeyModel.toBuffer().toString('hex');
    const publicKey = privateKeyModel.toPublicKey().toBuffer().toString('base64');
    publicKeyId = 1;

    stateTransition = new StateTransitionMock();

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);
  });

  describe('#toJSON', () => {
    it('should serialize data to JSON', () => {
      const serializedData = stateTransition.toJSON();

      expect(serializedData).to.deep.equal({
        signaturePublicKeyId: null,
        protocolVersion: 0,
        signature: null,
        type: 1,
      });
    });

    it('should serialize data to JSON without signature data', () => {
      const serializedData = stateTransition.toJSON({ skipSignature: true });

      expect(serializedData).to.deep.equal({
        protocolVersion: 0,
        type: 1,
      });
    });
  });

  describe('#hash', () => {
    it('should return serialized hash', () => {
      const hash = stateTransition.hash();

      expect(hash).to.be.equal('60fbcdd25bfd3581f476aa45341750fbd882a247e42cac2b9dcef89d862a97c4');
    });
  });

  describe('#serialize', () => {
    it('should return serialized data', () => {
      const serializedData = stateTransition.serialize();

      expect(serializedData.toString('hex')).to.be.equal('a4647479706501697369676e6174757265f66f70726f746f636f6c56657273696f6e00747369676e61747572655075626c69634b65794964f6');
    });

    it('should return serialized data without signature data', () => {
      const serializedData = stateTransition.serialize({ skipSignature: true });

      expect(serializedData.toString('hex')).to.be.equal('a26474797065016f70726f746f636f6c56657273696f6e00');
    });
  });

  describe('#getSignaturePublicKeyId', () => {
    it('should return public key ID', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      const keyId = stateTransition.getSignaturePublicKeyId();
      expect(keyId).to.be.equal(publicKeyId);
    });
  });

  describe('#getProtocolVersion', () => {
    it('should return protocol version', async () => {
      const protocolVersion = stateTransition.getProtocolVersion();

      expect(protocolVersion).to.be.equal(0);
    });
  });

  describe('#sign', () => {
    it('should sign data and validate signature with private key in hex format', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.a('string');

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should sign data and validate signature with private key in buffer format', () => {
      stateTransition.sign(identityPublicKey, privateKeyBuffer);

      expect(stateTransition.signature).to.be.a('string');

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an error if we try to sign with wrong public key', () => {
      const publicKey = new PrivateKey()
        .toPublicKey()
        .toBuffer()
        .toString('base64');

      identityPublicKey.setData(publicKey);

      try {
        stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw InvalidSignaturePublicKeyError');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidSignaturePublicKeyError);
        expect(e.getSignaturePublicKey()).to.be.equal(identityPublicKey.getData());
      }
    });

    it('should throw InvalidSignatureTypeError if signature type is not equal ECDSA', () => {
      identityPublicKey.setType(30000);

      try {
        stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw InvalidSignatureTypeError');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidSignatureTypeError);
        expect(e.getSignatureType()).to.be.equal(identityPublicKey.getType());
      }
    });
  });

  describe('#signByPrivateKey', () => {
    it('should sign and validate with private key', () => {
      privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

      stateTransition.signByPrivateKey(privateKeyHex);

      expect(stateTransition.signature).to.be.a('string');
    });
  });

  describe('#verifySignature', () => {
    it('should validate signature', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.a('string');

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', () => {
      try {
        stateTransition.verifySignature(identityPublicKey);

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });

    it('should throw an PublicKeyMismatchError error if public key id not equals public key id in state transition', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      identityPublicKey.setId(identityPublicKey.getId() + 1);

      try {
        stateTransition.verifySignature(identityPublicKey);

        expect.fail('should throw PublicKeyMismatchError');
      } catch (e) {
        expect(e).to.be.instanceOf(PublicKeyMismatchError);
        expect(e.getPublicKey()).to.equal(identityPublicKey);
      }
    });

    it('should not verify signature with wrong public key', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);
      const publicKey = new PrivateKey()
        .toPublicKey()
        .toBuffer()
        .toString('base64');

      identityPublicKey.setData(publicKey);

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.false();
    });
  });

  describe('#verifySignatureByPublicKey', () => {
    it('should validate sign by public key', () => {
      privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';
      const publicKey = 'A1eUrJ7lM6F1m6dbIyk+vXimKfzki+QRMHMwoAmggt6L';

      stateTransition.signByPrivateKey(privateKeyHex);

      const isValid = stateTransition.verifySignatureByPublicKey(Buffer.from(publicKey, 'base64'));

      expect(isValid).to.be.true();
    });

    it('should throw an StateTransitionIsNotSignedError error if transition is not signed', () => {
      const publicKey = 'A1eUrJ7lM6F1m6dbIyk+vXimKfzki+QRMHMwoAmggt6L';
      try {
        stateTransition.verifySignatureByPublicKey(Buffer.from(publicKey, 'base64'));

        expect.fail('should throw StateTransitionIsNotSignedError');
      } catch (e) {
        expect(e).to.be.instanceOf(StateTransitionIsNotSignedError);
        expect(e.getStateTransition()).to.equal(stateTransition);
      }
    });
  });

  describe('#setSignature', () => {
    it('should set signature', () => {
      const signature = 'signature';
      stateTransition.setSignature(signature);

      expect(stateTransition.signature).to.equal(signature);
    });
  });

  describe('#setSignaturePublicKeyId', () => {
    it('should set signature public key id', async () => {
      const signaturePublicKeyId = 1;
      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      expect(stateTransition.signaturePublicKeyId).to.equal(signaturePublicKeyId);
    });
  });
});
