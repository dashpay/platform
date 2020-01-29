const rewiremock = require('rewiremock/node');

const Identity = require('../../../../../lib/identity/Identity');
const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

describe('IdentityCreateTransition', () => {
  let rawStateTransition;
  let stateTransition;
  let hashMock;
  let signerMock;
  let IdentityCreateTransition;

  beforeEach(function beforeEach() {
    rawStateTransition = {
      lockedOutPoint: 'c3BlY2lhbEJ1ZmZlcg==',
      identityType: Identity.TYPES.USER,
      publicKeys: [
        {
          id: 1,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: 'someString',
          isEnabled: true,
        },
      ],
    };

    hashMock = this.sinonSandbox.stub();
    hashMock.returns(Buffer.alloc(32));

    signerMock = {
      sign: this.sinonSandbox.stub(),
      verifySignature: this.sinonSandbox.stub(),
    };

    IdentityCreateTransition = rewiremock.proxy(
      '../../../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition',
      {
        '../../../../../lib/util/hash': hashMock,
        '../../../../../node_modules/@dashevo/dashcore-lib': {
          Signer: signerMock,
        },
      },
    );

    stateTransition = new IdentityCreateTransition(rawStateTransition);
  });

  describe('#constructor', () => {
    it('should create an instance with default values if nothing specified', () => {
      stateTransition = new IdentityCreateTransition();

      expect(stateTransition.identityType).to.be.undefined();
      expect(stateTransition.publicKeys).to.deep.equal([]);
    });

    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.identityType).to.equal(1);
      expect(stateTransition.lockedOutPoint).to.deep.equal(
        rawStateTransition.lockedOutPoint,
      );
      expect(stateTransition.publicKeys).to.deep.equal([
        new IdentityPublicKey(rawStateTransition.publicKeys[0]),
      ]);
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_CREATE);
    });
  });

  describe('#setLockedOutPoint', () => {
    it('should set locked OutPoint', () => {
      stateTransition.setLockedOutPoint(Buffer.alloc(42, 3));
      expect(stateTransition.lockedOutPoint).to.deep.equal(Buffer.alloc(42, 3));
    });

    it('should set `identityId`', () => {
      hashMock.reset();
      hashMock.returns(Buffer.alloc(32));

      stateTransition = new IdentityCreateTransition();
      stateTransition.setLockedOutPoint(Buffer.alloc(0).toString('base64'));

      expect(hashMock).to.have.been.calledOnceWith(Buffer.alloc(0));
      expect(stateTransition.identityId).to.equal('11111111111111111111111111111111');
    });
  });

  describe('#getLockedOutPoint', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getLockedOutPoint()).to.deep.equal(
        rawStateTransition.lockedOutPoint,
      );
    });
  });

  describe('#setIdentityType', () => {
    it('should set identity type', () => {
      stateTransition.setIdentityType(42);
      expect(stateTransition.identityType).to.equal(42);
    });
  });

  describe('#getIdentityType', () => {
    it('should return identity type', () => {
      expect(stateTransition.getIdentityType()).to.equal(1);
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.setPublicKeys(publicKeys);

      expect(stateTransition.publicKeys).to.have.deep.members(publicKeys);
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(stateTransition.getPublicKeys()).to.deep.equal(
        rawStateTransition.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#addPublicKeys', () => {
    it('should add more public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.publicKeys = [];
      stateTransition.addPublicKeys(publicKeys);
      expect(stateTransition.getPublicKeys()).to.have.deep.members(publicKeys);
    });
  });

  describe('#getIdentityId', () => {
    it('should return set identity id', () => {
      expect(stateTransition.getIdentityId()).to.equal(
        '11111111111111111111111111111111',
      );
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of the object', () => {
      const jsonWithASig = stateTransition.toJSON();

      expect(jsonWithASig).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_CREATE,
        identityType: Identity.TYPES.USER,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        publicKeys: rawStateTransition.publicKeys,
        signature: null,
        signaturePublicKeyId: null,
      });

      const jsonWithSig = stateTransition.toJSON({ skipSignature: true });

      expect(jsonWithSig).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_CREATE,
        identityType: Identity.TYPES.USER,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        publicKeys: rawStateTransition.publicKeys,
      });
    });
  });
});
