const fetchProTxInfoFactory = require('../../../lib/core/fetchProTxInfoFactory');

describe('fetchProTxInfoFactory', () => {
  let fetchProTxInfo;
  let rpcClient;
  let proTxInfo;
  let proTxHash;

  beforeEach(function beforeEach() {
    proTxHash = '542b5ba206d2b30a366b6f6d0cf5e877816d7a252f984a6d920134091b9b11d0';
    proTxInfo = {
      proTxHash: '542b5ba206d2b30a366b6f6d0cf5e877816d7a252f984a6d920134091b9b11d0',
      collateralHash: 'e6a9a31b5cf64f457ff9c15fb076f7e9a758683a516c0aeb39ab4ce40c5496cf',
      collateralIndex: 0,
      collateralAddress: 'ySV123ZBEv8FsRC6NnkosB6vZodWTpgaBL',
      operatorReward: 0,
      state: {
        service: '192.168.65.2:20201',
        registeredHeight: 629,
        lastPaidHeight: 1106,
        PoSePenalty: 0,
        PoSeRevivedHeight: -1,
        PoSeBanHeight: -1,
        revocationReason: 0,
        ownerAddress: 'yUVs18ALrCc5CSLpvC9UeJ7tHjtdt5eW6a',
        votingAddress: 'yUVs18ALrCc5CSLpvC9UeJ7tHjtdt5eW6a',
        payoutAddress: 'yZVHb8YQtCkw5QYiHy3uRTQy9VCKbEg4Xe',
        pubKeyOperator: '120899de98537efc4d628c258a51f7f4f550360b1f93bc055c2b4238eb48be02ff00308a5ce3641e6410df9cafb89d7f',
      },
      confirmations: 481,
      metaInfo: {
        lastDSQ: 0,
        mixingTxCount: 0,
        lastOutboundAttempt: 1639781982,
        lastOutboundAttemptElapsed: -150,
        lastOutboundSuccess: 0,
        lastOutboundSuccessElapsed: 1639781832,
      },
    };

    rpcClient = {
      protx: this.sinon.stub().resolves({ result: proTxInfo }),
    };

    fetchProTxInfo = fetchProTxInfoFactory(rpcClient);
  });

  it('should fetch proTxInfo', async () => {
    const result = await fetchProTxInfo(proTxHash);

    expect(result).to.deep.equal(proTxInfo);
  });

  it('should throw custom error on code -8', async () => {
    const error = new Error('Not found');
    error.code = -8;

    rpcClient.protx.throws(error);

    try {
      await fetchProTxInfo(proTxHash);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.equal(`Protx with hash ${proTxHash} was not found`);
    }
  });

  it('should throw unknown error', async () => {
    const error = new Error('Not found');

    rpcClient.protx.throws(error);

    try {
      await fetchProTxInfo(proTxHash);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });
});
