const { expect } = require('chai');
const Validator = require('../../../lib/validator/Validator');
const ValidatorNetworkInfo = require('../../../lib/validator/ValidatorNetworkInfo');

describe('Validator', () => {
  let networkInfo;
  let host;
  let port;

  beforeEach(() => {
    host = '192.168.65.2';
    port = 26656;
    networkInfo = new ValidatorNetworkInfo(host, port);
  });

  describe('#createFromQuorumMember', () => {
    it('should create an instance from quorum member info', () => {
      const memberInfo = {
        proTxHash: Buffer.alloc(0, 32).toString('hex'),
        pubKeyShare: Buffer.alloc(1, 32).toString('hex'),
      };

      const instance = Validator.createFromQuorumMember(memberInfo, networkInfo);

      expect(instance).to.be.an.instanceOf(Validator);
      expect(instance.getProTxHash()).to.deep.equal(
        Buffer.from(memberInfo.proTxHash, 'hex'),
      );
      expect(instance.getPublicKeyShare()).to.deep.equal(
        Buffer.from(memberInfo.pubKeyShare, 'hex'),
      );
      expect(instance.getNetworkInfo()).to.be.an.instanceOf(ValidatorNetworkInfo);
      expect(instance.getNetworkInfo().getHost()).to.equal(host);
      expect(instance.getNetworkInfo().getPort()).to.equal(port);
    });
  });
});
