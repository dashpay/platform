const { PrivateKey } = require('@dashevo/dashcore-lib');

const calculateStateTransitionFee = require('../../../lib/stateTransition/calculateStateTransitionFee');

const StateTransitionMock = require('../../../lib/test/mocks/StateTransitionMock');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const InvalidSignatureTypeError = require('../../../lib/stateTransition/errors/InvalidSignatureTypeError');
const InvalidSignaturePublicKeyError = require('../../../lib/stateTransition/errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('../../../lib/stateTransition/errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('../../../lib/stateTransition/errors/PublicKeyMismatchError');
const EncodedBuffer = require('../../../lib/util/encoding/EncodedBuffer');

describe('AbstractStateTransitionIdentitySigned', () => {
  let stateTransition;
  let protocolVersion;
  let privateKeyHex;
  let privateKeyBuffer;
  let publicKeyId;
  let identityPublicKey;

  beforeEach(() => {
    const privateKeyModel = new PrivateKey();
    privateKeyBuffer = privateKeyModel.toBuffer();
    privateKeyHex = privateKeyModel.toBuffer().toString('hex');
    const publicKey = privateKeyModel.toPublicKey().toBuffer();
    publicKeyId = 1;

    protocolVersion = 1;

    stateTransition = new StateTransitionMock({
      protocolVersion,
    });

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);
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
    it('should return public key ID', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      const keyId = stateTransition.getSignaturePublicKeyId();
      expect(keyId).to.be.equal(publicKeyId);
    });
  });

  describe('#sign', () => {
    it('should sign data and validate signature with private key in hex format', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(EncodedBuffer);

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should sign data and validate signature with private key in buffer format', () => {
      stateTransition.sign(identityPublicKey, privateKeyBuffer);

      expect(stateTransition.signature).to.be.an.instanceOf(EncodedBuffer);

      const isValid = stateTransition.verifySignature(identityPublicKey);

      expect(isValid).to.be.true();
    });

    it('should throw an error if we try to sign with wrong public key', () => {
      const publicKey = new PrivateKey()
        .toPublicKey()
        .toBuffer();

      identityPublicKey.setData(publicKey);

      try {
        stateTransition.sign(identityPublicKey, privateKeyHex);

        expect.fail('Should throw InvalidSignaturePublicKeyError');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidSignaturePublicKeyError);
        expect(e.getSignaturePublicKey()).to.deep.equal(identityPublicKey.getData());
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

      expect(stateTransition.signature).to.be.an.instanceOf(EncodedBuffer);
    });
  });

  describe('#verifySignature', () => {
    it('should validate signature', () => {
      stateTransition.sign(identityPublicKey, privateKeyHex);

      expect(stateTransition.signature).to.be.an.instanceOf(EncodedBuffer);

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
        .toBuffer();

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
