const { expect } = require('chai');
const getScoredQuorumHashes = require('../../../lib/core/getScoredQuorumHashes');

describe('getScoredQuorumHashes', () => {
  let smlMock;
  let quorumType;

  beforeEach(function beforeEach() {
    smlMock = {
      getQuorumsOfType: this.sinon.stub(),
      quorumList: [],
      blockHash: '0'.repeat(32),
    };

    quorumType = 1;

    smlMock.getQuorumsOfType.returns([
      {
        quorumHash: Buffer.alloc(1, 32).toString('hex'),
      },
    ]);
  });

  it('should return random quorum based on entropy', () => {
    const result = getScoredQuorumHashes(smlMock, quorumType, Buffer.alloc(1));

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);

    expect(result).to.deep.equal([{
      hash: Buffer.alloc(1, 32),
      score: Buffer.from('869f1dfb999a452f497a4cf7f44db2d6ee661f74a9e7e05251bc1420e50672d4', 'hex'),
    }]);
  });

  it('should throw an error if SML does not contain any quorums', () => {
    smlMock.getQuorumsOfType.returns([]);

    expect(() => {
      getScoredQuorumHashes(smlMock, quorumType, Buffer.alloc(1));
    }).to.throw(`SML at block ${'0'.repeat(32)} contains no quorums of any type`);
  });

  it('should throw an error if SML contains quorums that differ from the specified quorum type', () => {
    smlMock.getQuorumsOfType.returns([]);
    smlMock.quorumList = [{ llmqType: 999 }];

    expect(() => {
      getScoredQuorumHashes(smlMock, quorumType, Buffer.alloc(1));
    }).to.throw(`SML at block ${'0'.repeat(32)} contains no quorums of type 1, but contains entries for types 999. Please check the Drive configuration`);
  });
});
