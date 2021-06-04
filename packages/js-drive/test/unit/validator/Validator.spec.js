const { expect } = require('chai');
const Validator = require('../../../lib/validator/Validator');

describe('Validator', () => {
  describe('createFromQuorumMember', () => {
    it('should create an instance from quorum member info', () => {
      const memberInfo = {
        proTxHash: Buffer.alloc(0, 32).toString('hex'),
        pubKeyShare: Buffer.alloc(1, 32).toString('hex'),
      };

      const instance = Validator.createFromQuorumMember(memberInfo);

      expect(instance).to.be.an.instanceOf(Validator);
      expect(instance.getProTxHash()).to.deep.equal(
        Buffer.from(memberInfo.proTxHash, 'hex'),
      );
      expect(instance.getPublicKeyShare()).to.deep.equal(
        Buffer.from(memberInfo.pubKeyShare, 'hex'),
      );
    });
  });
});
