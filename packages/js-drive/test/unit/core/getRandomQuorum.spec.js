const { QuorumEntry } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const getRandomQuorum = require('../../../lib/core/getRandomQuorum');

describe('getRandomQuorum', () => {
  let smlMock;
  let quorumType;
  let randomQuorum;

  beforeEach(function beforeEach() {
    smlMock = {
      getQuorumsOfType: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
      quorumList: [],
      blockHash: '0'.repeat(32),
    };

    quorumType = 1;

    smlMock.getQuorumsOfType.returns([
      {
        quorumHash: Buffer.alloc(1, 32).toString('hex'),
      },
    ]);

    randomQuorum = new QuorumEntry();

    smlMock.getQuorum.returns(randomQuorum);
  });

  it('should return random quorum based on entropy', () => {
    const result = getRandomQuorum(smlMock, quorumType, Buffer.alloc(1));

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
    expect(smlMock.getQuorum).to.have.been.calledOnceWithExactly(
      quorumType, '20',
    );
    expect(result).to.equals(randomQuorum);
  });

  it('should throw an error if SML does not contain any quorums', () => {
    smlMock.getQuorumsOfType.returns([]);

    expect(() => {
      getRandomQuorum(smlMock, quorumType, Buffer.alloc(1));
    }).to.throw(`SML at block ${'0'.repeat(32)} contains no quorums of any type`);
  });

  it('should throw an error if SML contains quorums that differ from the specified quorum type', () => {
    smlMock.getQuorumsOfType.returns([]);
    smlMock.quorumList = [{ llmqType: 999 }];

    expect(() => {
      getRandomQuorum(smlMock, quorumType, Buffer.alloc(1));
    }).to.throw(`SML at block ${'0'.repeat(32)} contains no quorums of type 1, but contains entries for types 999. Please check the Drive configuration`);
  });
});
