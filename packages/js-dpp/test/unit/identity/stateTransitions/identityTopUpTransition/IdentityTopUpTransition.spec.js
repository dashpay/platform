const rewiremock = require('rewiremock/node');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

describe('IdentityTopUpTransition', () => {
  let rawStateTransition;
  let stateTransition;
  let hashMock;
  let signerMock;
  let IdentityTopUpTransition;
  let identityTopUpTransition;

  beforeEach(function beforeEach() {
    identityTopUpTransition = getIdentityTopUpTransitionFixture();
    rawStateTransition = identityTopUpTransition.toObject();

    hashMock = this.sinonSandbox.stub();
    hashMock.returns(Buffer.alloc(32));

    signerMock = {
      signByPrivateKey: this.sinonSandbox.stub(),
      verifySignatureByPublicKey: this.sinonSandbox.stub(),
    };

    IdentityTopUpTransition = rewiremock.proxy(
      '../../../../../lib/identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition',
      {
        '../../../../../lib/util/hash': hashMock,
        '../../../../../node_modules/@dashevo/dashcore-lib': {
          Signer: signerMock,
        },
      },
    );

    stateTransition = new IdentityTopUpTransition(rawStateTransition);
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.getLockedOutPoint()).to.be.deep.equal(
        rawStateTransition.lockedOutPoint,
      );
      expect(stateTransition.getIdentityId()).to.be.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_TOP_UP);
    });
  });

  describe('#setLockedOutPoint', () => {
    it('should set locked OutPoint', () => {
      stateTransition.setLockedOutPoint(Buffer.alloc(42, 3));
      expect(stateTransition.lockedOutPoint).to.deep.equal(Buffer.alloc(42, 3));
    });
  });

  describe('#getLockedOutPoint', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getLockedOutPoint()).to.deep.equal(
        rawStateTransition.lockedOutPoint,
      );
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        identityId: rawStateTransition.identityId,
        signature: undefined,
      });
    });

    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        identityId: rawStateTransition.identityId,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        lockedOutPoint: rawStateTransition.lockedOutPoint.toString('base64'),
        identityId: Identifier(rawStateTransition.identityId).toString(),
        signature: undefined,
      });
    });
  });
});
