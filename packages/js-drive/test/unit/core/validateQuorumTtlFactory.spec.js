const QuorumEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/QuorumEntry');
const validateQuorumTtlFactory = require('../../../lib/core/validateQuorumTtlFactory');
const getSmlFixture = require('../../../lib/test/fixtures/getSmlFixture');

describe('validateQuorumTtlFactory', () => {
  let validateQuorumTtl;
  let coreRpcClientMock;
  let blockRotationInterval;
  let coreHeight;
  let quorumEntry;
  let smlMock;
  let quorumType;

  beforeEach(function beforeEach() {
    blockRotationInterval = 15;
    coreHeight = 42;

    quorumEntry = new QuorumEntry(getSmlFixture()[0].newQuorums[0]);

    smlMock = {
      getQuorumsOfType: this.sinon.stub(),
    };

    coreRpcClientMock = {
      getBlock: this.sinon.stub(),
    };

    validateQuorumTtl = validateQuorumTtlFactory(coreRpcClientMock);

    quorumType = 1;

    smlMock.getQuorumsOfType.returns(new Array(1));
  });

  it('should return true is ttl is enough', async () => {
    coreRpcClientMock.getBlock.resolves({
      height: 41,
    });

    const result = await validateQuorumTtl(
      smlMock,
      quorumType,
      quorumEntry,
      coreHeight,
      blockRotationInterval,
    );

    expect(result).to.be.true();

    expect(coreRpcClientMock.getBlock).to.be.calledOnceWithExactly(quorumEntry.quorumHash);
    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
  });

  it('should return false is ttl is not enough', async () => {
    coreRpcClientMock.getBlock.resolves({
      height: 10,
    });

    const result = await validateQuorumTtl(
      smlMock,
      quorumType,
      quorumEntry,
      coreHeight,
      blockRotationInterval,
    );

    expect(result).to.be.false();

    expect(coreRpcClientMock.getBlock).to.be.calledOnceWithExactly(quorumEntry.quorumHash);
    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
  });
});
