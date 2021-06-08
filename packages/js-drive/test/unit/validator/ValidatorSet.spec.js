const Long = require('long');

const QuorumEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/QuorumEntry');

const ValidatorSet = require('../../../lib/validator/ValidatorSet');
const getSmlFixture = require('../../../lib/test/fixtures/getSmlFixture');
const ValidatorSetIsNotInitializedError = require('../../../lib/validator/errors/ValidatorSetIsNotInitializedError');
const Validator = require('../../../lib/validator/Validator');

describe('ValidatorSet', () => {
  let smlStoreMock;
  let simplifiedMasternodeListMock;
  let smlDiffMock;
  let smlMock;
  let quorumMembers;
  let rotationEntropy;
  let quorumEntry;
  let coreHeight;

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

    validatorSet = new ValidatorSet(
      simplifiedMasternodeListMock,
      getRandomQuorumMock,
      fetchQuorumMembersMock,
      validatorSetLLMQType,
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
