const getIdentityCreditTransferTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreditTransferTransitionFixture');

const {
  Identifier, StateTransitionTypes,
} = require('../../../../..');

describe('IdentityCreditTransferTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(async () => {
    stateTransition = await getIdentityCreditTransferTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.getIdentityId().toBuffer()).to.be.deep.equal(
        rawStateTransition.identityId,
      );

      expect(stateTransition.getRecipientId().toBuffer()).to.be.deep.equal(
        rawStateTransition.recipientId,
      );

      expect(stateTransition.getAmount()).to.be.equal(
        rawStateTransition.amount,
      );
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREDIT_TRANSFER type', () => {
      expect(stateTransition.getType()).to.equal(StateTransitionTypes.IdentityCreditTransfer);
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getRecipientId', () => {
    it('should return recipient id', () => {
      expect(stateTransition.getRecipientId().toBuffer()).to.deep.equal(
        rawStateTransition.recipientId,
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        $version: '0',
        type: StateTransitionTypes.IdentityCreditTransfer,
        identityId: rawStateTransition.identityId,
        recipientId: rawStateTransition.recipientId,
        amount: rawStateTransition.amount,
        signature: undefined,
        signaturePublicKeyId: undefined,
      });
    });

    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        $version: '0',
        type: StateTransitionTypes.IdentityCreditTransfer,
        identityId: rawStateTransition.identityId,
        recipientId: rawStateTransition.recipientId,
        amount: rawStateTransition.amount,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        $version: '0',
        type: StateTransitionTypes.IdentityCreditTransfer,
        identityId: new Identifier(rawStateTransition.identityId).toString(),
        recipientId: new Identifier(rawStateTransition.recipientId).toString(),
        amount: rawStateTransition.amount,
        signature: undefined,
        signaturePublicKeyId: undefined,
      });
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of topped up identity', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(2);
      const [identityId, recipientId] = result;

      expect(identityId).to.be.an.instanceOf(Identifier);
      expect(identityId.toBuffer()).to.be.deep.equal(rawStateTransition.identityId);

      expect(recipientId).to.be.an.instanceOf(Identifier);
      expect(recipientId.toBuffer()).to.be.deep.equal(rawStateTransition.recipientId);
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.true();
    });
  });
});
