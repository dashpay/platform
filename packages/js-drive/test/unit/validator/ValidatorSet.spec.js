const Long = require('long');

const QuorumEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/QuorumEntry');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const ValidatorSet = require('../../../lib/validator/ValidatorSet');
const getSmlFixture = require('../../../lib/test/fixtures/getSmlFixture');
const ValidatorSetIsNotInitializedError = require('../../../lib/validator/errors/ValidatorSetIsNotInitializedError');
const Validator = require('../../../lib/validator/Validator');
const PublicKeyShareIsNotPresentError = require('../../../lib/validator/errors/PublicKeyShareIsNotPresentError');

describe('ValidatorSet', () => {
  let smlStoreMock;
  let simplifiedMasternodeListMock;
  let smlDiffMock;
  let smlMock;
  let quorumMembers;
  let rotationEntropy;
  let quorumEntry;
  let coreHeight;
  let coreRpcClientMock;
  let validatorNetworkPort;

  let validatorSetLLMQType;

  let validatorSet;
  let getRandomQuorumMock;
  let fetchQuorumMembersMock;

  beforeEach(function beforeEach() {
    coreHeight = 42;

    validatorSetLLMQType = 4;

    quorumEntry = new QuorumEntry(getSmlFixture()[0].newQuorums[0]);

    smlDiffMock = {
      blockHash: 'some block hash',
    };

    smlMock = {
      getQuorum: this.sinon.stub().returns(quorumEntry),
      toSimplifiedMNListDiff: this.sinon.stub().returns(smlDiffMock),
      getQuorumsOfType: this.sinon.stub().returns(
        getSmlFixture()[0].newQuorums.filter((quorum) => quorum.llmqType === 1),
      ),
      getValidMasternodesList: this.sinon.stub().returns([
        new SimplifiedMNListEntry({
          proRegTxHash: 'c286807d463b06c7aba3b9a60acf64c1fc03da8c1422005cd9b4293f08cf0562',
          confirmedHash: '4eb56228c535db3b234907113fd41d57bcc7cdcb8e0e00e57590af27ee88c119',
          service: '192.168.65.2:20101',
          pubKeyOperator: '809519c5f6f3be1c08782ac42ae9a83b6c7205eba43f9a96a4f032ec7a73f1a7c25fa78cce0d6d9c135f7e2c28527179',
          votingAddress: 'yXmprXYP51uzfMyndtWwxz96MnkCKkFc9x',
          isValid: true,
        }),
        new SimplifiedMNListEntry({
          proRegTxHash: 'a3e1edc6bd352eeaf0ae58e30781ef4b127854241a3fe7fddf36d5b7e1dc2b3f',
          confirmedHash: '27a0b637b56af038c45e2fd1f06c2401c8dadfa28ca5e0d19ca836cc984a8378',
          service: '192.168.65.2:20201',
          pubKeyOperator: '987a4873caba62cd45a2f7d4aa6d94519ee6753e9bef777c927cb94ade768a542b0ff34a93231d3a92b4e75ffdaa366e',
          votingAddress: 'ycL7L4mhYoaZdm9TH85svvpfeKtdfo249u',
          isValid: true,
        }),
      ]),
    };

    smlStoreMock = {
      getSMLbyHeight: this.sinon.stub().returns(smlMock),
      getCurrentSML: this.sinon.stub().returns(smlMock),
    };

    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub().returns(smlStoreMock),
    };

    rotationEntropy = Buffer.from('00000ac05a06682172d8b49be7c9ddc4189126d7200ebf0fc074c433ae74b596', 'hex');

    quorumMembers = [
      {
        proTxHash: 'c286807d463b06c7aba3b9a60acf64c1fc03da8c1422005cd9b4293f08cf0562',
        pubKeyOperator: '06abc1c890c9da4e513d52f20da1882228bfa2db4bb29cbd064e1b2a61d9dcdadcf0784fd1371338c8ad1bf323d87ae6',
        valid: true,
        pubKeyShare: '00d7bb8d6753865c367824691610dcc313b661b7e024e36e82f8af33f5701caddb2668dadd1e647d8d7d5b30e37ebbcf',
      },
      {
        proTxHash: 'a3e1edc6bd352eeaf0ae58e30781ef4b127854241a3fe7fddf36d5b7e1dc2b3f',
        pubKeyOperator: '04d748ba0efeb7a8f8548e0c22b4c188c293a19837a1c5440649279ba73ead0c62ac1e840050a10a35e0ae05659d2a8d',
        valid: true,
        pubKeyShare: '86d0992f5c73b8f57101c34a0c4ebb17d962bb935a738c1ef1e2bb1c25034d8e4a0a2cc96e0ebc69a7bf3b8b67b2de5f',
      },
      {
        proTxHash: 'a3e1edc6bd352eeaf0ae58e30781ef4b127854241a3fe7fddf36d5b7e1dc2b3f',
        pubKeyOperator: '04d748ba0efeb7a8f8548e0c22b4c188c293a19837a1c5440649279ba73ead0c62ac1e840050a10a35e0ae05659d2a8d',
        valid: false,
      },
    ];

    getRandomQuorumMock = this.sinon.stub().resolves(quorumEntry);

    fetchQuorumMembersMock = this.sinon.stub().resolves(quorumMembers);

    const notMasternodeError = new Error();
    notMasternodeError.code = -32603;

    coreRpcClientMock = {
      masternode: this.sinon.stub().throws(notMasternodeError),
    };

    validatorNetworkPort = 26656;

    validatorSet = new ValidatorSet(
      simplifiedMasternodeListMock,
      getRandomQuorumMock,
      fetchQuorumMembersMock,
      validatorSetLLMQType,
      coreRpcClientMock,
      validatorNetworkPort,
    );
  });

  describe('initialize', () => {
    it('should initialize with specified core height', async () => {
      await validatorSet.initialize(coreHeight);

      expect(smlStoreMock.getSMLbyHeight).to.be.calledOnceWithExactly(coreHeight);

      expect(getRandomQuorumMock).to.be.calledOnceWithExactly(
        smlMock,
        validatorSetLLMQType,
        Buffer.from(smlDiffMock.blockHash, 'hex'),
      );

      expect(fetchQuorumMembersMock).to.be.calledOnceWithExactly(
        validatorSetLLMQType,
        quorumEntry.quorumHash,
      );

      expect(smlStoreMock.getCurrentSML().getValidMasternodesList).to.be.calledOnce();
    });

    it('should throw an error if the node is a quorum member and doesn\'t receive public key shares', async () => {
      coreRpcClientMock.masternode.resolves({
        result: {
          proTxHash: quorumMembers[0].proTxHash,
        },
      });

      quorumMembers[2].valid = true;

      try {
        await validatorSet.initialize(coreHeight);

        expect.fail('should throw PublicKeyShareIsNotPresentError');
      } catch (e) {
        expect(e).to.be.instanceOf(PublicKeyShareIsNotPresentError);
        expect(e.getMember()).to.be.equal(quorumMembers[2]);
      }
    });
  });

  describe('rotate', () => {
    it('should rotate validator set with specified core height and entropy if height divisible by ROTATION_BLOCK_INTERVAL', async () => {
      const height = Long.fromInt(ValidatorSet.ROTATION_BLOCK_INTERVAL);

      const result = await validatorSet.rotate(
        height,
        coreHeight,
        rotationEntropy,
      );

      expect(result).to.be.true();

      expect(smlStoreMock.getSMLbyHeight).to.be.calledOnceWithExactly(coreHeight);

      expect(getRandomQuorumMock).to.be.calledOnceWithExactly(
        smlMock,
        validatorSetLLMQType,
        rotationEntropy,
      );

      expect(fetchQuorumMembersMock).to.be.calledOnceWithExactly(
        validatorSetLLMQType,
        quorumEntry.quorumHash,
      );

      expect(smlStoreMock.getCurrentSML().getValidMasternodesList).to.be.calledOnce();
    });

    it('should not rotate validator set if height not divisible by ROTATION_BLOCK_INTERVAL', async () => {
      const height = Long.fromInt(42);

      const result = await validatorSet.rotate(
        height,
        coreHeight,
        rotationEntropy,
      );

      expect(result).to.be.false();

      expect(smlStoreMock.getSMLbyHeight).to.be.calledOnceWithExactly(coreHeight);

      expect(getRandomQuorumMock).to.not.be.called();

      expect(fetchQuorumMembersMock).to.not.be.called();
    });
  });

  describe('getQuorum', () => {
    it('should return QuorumEntry', async () => {
      await validatorSet.initialize(coreHeight);

      const result = validatorSet.getQuorum();

      expect(result).to.equals(quorumEntry);
    });

    it('should thrown an error if ValidatorSet is not initialized', () => {
      try {
        validatorSet.getQuorum();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(ValidatorSetIsNotInitializedError);
      }
    });
  });

  describe('getValidators', () => {
    it('should return array of validators', async () => {
      await validatorSet.initialize(coreHeight);

      const result = validatorSet.getValidators();

      expect(result).to.have.lengthOf(2);
      expect(result[0]).to.be.instanceOf(Validator);
      expect(result[1]).to.be.instanceOf(Validator);
    });

    it('should thrown an error if ValidatorSet is not initialized', () => {
      try {
        validatorSet.getValidators();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(ValidatorSetIsNotInitializedError);
      }
    });
  });
});
