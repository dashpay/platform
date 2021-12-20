const { expect } = require('chai');
const fetchNetworkInfoFactory = require('../../../lib/validator/fetchNetworkInfoFactory');
const ValidatorNetworkInfo = require('../../../lib/validator/ValidatorNetworkInfo');

describe('fetchNetworkInfoFactory', () => {
  let fetchNetworkInfo;
  let p2pPort;
  let fetchProTxInfoMock;
  let proTxInfo;

  beforeEach(function beforeEach() {
    proTxInfo = {
      proTxHash: '711e53894c649913b3406624c66bdb302ff1263fe692c18677309beba2f3d85d',
      collateralHash: '03da4a14589265ab010b24559e2916a40ac3c50786ed15d1ec6b6ce895fb2eda',
      collateralIndex: 0,
      collateralAddress: 'yTAsMMJaewChEevGKpVpPNkMYgHxhJXw9c',
      operatorReward: 0,
      state: {
        service: '192.168.65.2:20101',
        registeredHeight: 626,
        lastPaidHeight: 994,
        PoSePenalty: 0,
        PoSeRevivedHeight: -1,
        PoSeBanHeight: -1,
        revocationReason: 0,
        ownerAddress: 'yanL8fcS9v1KbfoAprcpY6kF1FyBoiHRZK',
        votingAddress: 'yanL8fcS9v1KbfoAprcpY6kF1FyBoiHRZK',
        payoutAddress: 'yVy5mAY9eKYV1UUtTJHNumycBMMq2Wy5td',
        pubKeyOperator: '90b63efdc888f2f3e30ffe5c971f4faaaca3fb9211e9ad05890a64905e52cc4dbd2f5fe37c7a7177f96ff1853981c5d5',
      },
      confirmations: 372,
      metaInfo: {
        lastDSQ: 0,
        mixingTxCount: 0,
        lastOutboundAttempt: 1639767132,
        lastOutboundAttemptElapsed: -600,
        lastOutboundSuccess: 0,
        lastOutboundSuccessElapsed: 1639766532,
      },
    };

    fetchProTxInfoMock = this.sinon.stub().resolves(proTxInfo);
    p2pPort = 26656;

    fetchNetworkInfo = fetchNetworkInfoFactory(fetchProTxInfoMock, p2pPort);
  });

  it('should return ValidatorNetworkInfo', async () => {
    const quorumMember = {
      proTxHash: '542b5ba206d2b30a366b6f6d0cf5e877816d7a252f984a6d920134091b9b11d0',
      pubKeyOperator: '120899de98537efc4d628c258a51f7f4f550360b1f93bc055c2b4238eb48be02ff00308a5ce3641e6410df9cafb89d7f',
      valid: true,
      pubKeyShare: '83e920c320fbff00e67a9c93f8678fbfc474e0ac4ea98e9b2ea9ea2df8c6fcfa058b4a73342452504c77499ba2ff318d',
    };

    const result = await fetchNetworkInfo(quorumMember);

    expect(result).to.be.an.instanceOf(ValidatorNetworkInfo);
    expect(fetchProTxInfoMock).to.be.calledOnceWithExactly(quorumMember.proTxHash);

    expect(result.getPort()).to.equal(p2pPort);
    expect(result.getHost()).to.equal('192.168.65.2');
  });
});
