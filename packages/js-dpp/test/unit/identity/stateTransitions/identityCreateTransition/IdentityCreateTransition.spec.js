const rewiremock = require('rewiremock/node');

const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const Identity = require('../../../../../lib/identity/Identity');

describe('IdentityCreateTransition', () => {
  let rawStateTransition;
  let stateTransition;
  let hashMock;
  let signerMock;
  let IdentityCreateTransition;

  beforeEach(function beforeEach() {
    rawStateTransition = {
      protocolVersion: Identity.PROTOCOL_VERSION,
      lockedOutPoint: 'c3BlY2lhbEJ1ZmZlcg==',
      publicKeys: [
        {
          id: 0,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: 'someString',
          isEnabled: true,
        },
      ],
    };

    hashMock = this.sinonSandbox.stub();
    hashMock.returns(Buffer.alloc(32));

    signerMock = {
      signByPrivateKey: this.sinonSandbox.stub(),
      verifySignatureByPublicKey: this.sinonSandbox.stub(),
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

      expect(stateTransition.publicKeys).to.deep.equal([]);
    });

    it('should create an instance with specified data from specified raw transition', () => {
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
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId()).to.equal(
        '11111111111111111111111111111111',
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.equal(
        stateTransition.getIdentityId(),
      );
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of the object', () => {
      const jsonWithASig = stateTransition.toJSON();

      expect(jsonWithASig).to.deep.equal({
        protocolVersion: Identity.PROTOCOL_VERSION,
        type: stateTransitionTypes.IDENTITY_CREATE,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        publicKeys: rawStateTransition.publicKeys,
        signature: null,
      });

      const jsonWithSig = stateTransition.toJSON({ skipSignature: true });

      expect(jsonWithSig).to.deep.equal({
        protocolVersion: Identity.PROTOCOL_VERSION,
        type: stateTransitionTypes.IDENTITY_CREATE,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        publicKeys: rawStateTransition.publicKeys,
      });
    });
  });
});
