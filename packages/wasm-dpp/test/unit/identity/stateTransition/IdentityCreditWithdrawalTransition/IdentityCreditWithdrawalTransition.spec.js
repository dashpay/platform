const getIdentityCreditWithdrawalTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreditWithdrawalTransitionFixture');

const {
  default: loadWasmDpp,
  Identifier, StateTransitionTypes,
} = require('../../../../..');

describe('IdentityCreditWithdrawalTransition', () => {
  let rawStateTransition;
  let stateTransition;

  before(loadWasmDpp);

  beforeEach(async () => {
    stateTransition = await getIdentityCreditWithdrawalTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREDIT_WITHDRAWAL type', () => {
      expect(stateTransition.getType()).to.equal(StateTransitionTypes.IdentityCreditWithdrawal);
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getAmount', () => {
    it('should return amount', () => {
      expect(stateTransition.getAmount()).to.be.equal(
        rawStateTransition.amount,
      );
    });
  });

  describe('#getCoreFeePerByte', () => {
    it('should return core fee per byte', () => {
      expect(stateTransition.getCoreFeePerByte()).to.be.equal(
        rawStateTransition.coreFeePerByte,
      );
    });
  });

  describe('#getPooling', () => {
    it('should return pooling', () => {
      expect(stateTransition.getPooling()).to.be.equal(
        rawStateTransition.pooling,
      );
    });
  });

  describe('#getOutputScript', () => {
    it('should return output script', () => {
      expect(stateTransition.getOutputScript()).to.be.deep.equal(
        rawStateTransition.outputScript,
      );
    });
  });

  describe('#getNonce', () => {
    it('should return revision', () => {
      expect(stateTransition.getNonce()).to.be.equal(
        rawStateTransition.nonce,
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        $version: '1',
        type: StateTransitionTypes.IdentityCreditWithdrawal,
        identityId: stateTransition.getIdentityId().toBuffer(),
        amount: stateTransition.getAmount(),
        coreFeePerByte: stateTransition.getCoreFeePerByte(),
        pooling: stateTransition.getPooling(),
        outputScript: stateTransition.getOutputScript(),
        nonce: stateTransition.getNonce(),
        signature: undefined,
        signaturePublicKeyId: undefined,
      });
    });

    it('should return raw state transition without signature', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        $version: '1',
        type: StateTransitionTypes.IdentityCreditWithdrawal,
        identityId: stateTransition.getIdentityId().toBuffer(),
        amount: stateTransition.getAmount(),
        coreFeePerByte: stateTransition.getCoreFeePerByte(),
        pooling: stateTransition.getPooling(),
        outputScript: stateTransition.getOutputScript(),
        nonce: stateTransition.getNonce(),
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        $version: '1',
        type: StateTransitionTypes.IdentityCreditWithdrawal,
        identityId: stateTransition.getIdentityId().toString(),
        amount: stateTransition.getAmount().toString(),
        coreFeePerByte: stateTransition.getCoreFeePerByte(),
        pooling: stateTransition.getPooling(),
        outputScript: stateTransition.getOutputScript().toString('base64'),
        nonce: stateTransition.getNonce().toString(),
        signature: undefined,
        signaturePublicKeyId: undefined,
      });
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of topped up identity', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const [identityId] = result;

      expect(identityId).to.be.an.instanceOf(Identifier);
      expect(identityId.toBuffer()).to.be.deep.equal(rawStateTransition.identityId);
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
