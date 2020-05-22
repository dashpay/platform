const rewiremock = require('rewiremock/node');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

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
    rawStateTransition = identityTopUpTransition.toJSON();

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
      expect(stateTransition.lockedOutPoint).to.be.equal(
        rawStateTransition.lockedOutPoint,
      );
      expect(stateTransition.identityId).to.be.equal(
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
      expect(stateTransition.getIdentityId()).to.equal(
        '9egkkRs6ErFbLUh3yYn8mdgeKGpJQ41iayS1Z9bwsRM7',
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
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        identityId: '9egkkRs6ErFbLUh3yYn8mdgeKGpJQ41iayS1Z9bwsRM7',
        signature: null,
      });

      const jsonWithSig = stateTransition.toJSON({ skipSignature: true });

      expect(jsonWithSig).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        lockedOutPoint: rawStateTransition.lockedOutPoint,
        identityId: '9egkkRs6ErFbLUh3yYn8mdgeKGpJQ41iayS1Z9bwsRM7',
      });
    });
  });
});
