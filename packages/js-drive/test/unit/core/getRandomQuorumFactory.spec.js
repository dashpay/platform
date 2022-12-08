const { expect } = require('chai');
const getRandomQuorumFactory = require('../../../lib/core/getRandomQuorumFactory');

describe('getRandomQuorumFactory', () => {
  let smlMock;
  let quorumType;
  let getRandomQuorum;
  let coreRpcClientMock;
  let randomQuorum;
  let coreHeight;

  beforeEach(function beforeEach() {
    smlMock = {
      getQuorumsOfType: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
      quorumList: [],
      blockHash: '0'.repeat(32),
    };

    coreHeight = 690;

    quorumType = 102;

    smlMock.getQuorum.resolves(randomQuorum);

    smlMock.getQuorumsOfType.returns([
      {
        quorumHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
        validMembersCount: 90,
      },
      {
        quorumHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
        getAllQuorumMembers: 90,
      },
    ]);

    const quorumListExtended = {
      llmq_test: [
        {
          '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c': {
            creationHeight: 672,
            minedBlockHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
          },
        },
        {
          '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce': {
            creationHeight: 672,
            minedBlockHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
          },
        },
      ],
      llmq_test_instantsend: [
        {
          '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c': {
            creationHeight: 672,
            minedBlockHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
          },
        },
        {
          '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce': {
            creationHeight: 672,
            minedBlockHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
          },
        },
      ],
      llmq_test_v17: [
        {
          '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c': {
            creationHeight: 672,
            minedBlockHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
          },
        },
        {
          '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce': {
            creationHeight: 672,
            minedBlockHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
          },
        },
      ],
    };

    coreRpcClientMock = {
      quorum: this.sinon.stub().resolves({ result: quorumListExtended }),
    };

    getRandomQuorum = getRandomQuorumFactory(coreRpcClientMock);
  });

  it('should return random quorum based on entropy', async () => {
    const result = await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
    expect(smlMock.getQuorum).to.have.been.calledOnceWithExactly(
      quorumType, '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
    );
    expect(result).to.equals(randomQuorum);
  });

  it('should throw an error if SML does not contain any quorums', async () => {
    smlMock.getQuorumsOfType.returns([]);

    try {
      await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.be.equal(`SML at block ${'0'.repeat(32)} contains no quorums of any type`);
    }
  });

  it('should throw an error if SML contains quorums that differ from the specified quorum type', async () => {
    smlMock.getQuorumsOfType.returns([]);
    smlMock.quorumList = [{ llmqType: 999 }];

    try {
      await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.be.equal(`SML at block ${'0'.repeat(32)} contains no quorums of type ${quorumType}, but contains entries for types 999. Please check the Drive configuration`);
    }
  });

  it('should filter quorums by minQuorumMembers', async () => {
    smlMock.getQuorumsOfType.returns([
      {
        quorumHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
        validMembersCount: 90,
      },
      {
        quorumHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
        validMembersCount: 89,
      },
    ]);

    const result = await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
    expect(smlMock.getQuorum).to.have.been.calledOnceWithExactly(
      quorumType, '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
    );
    expect(result).to.equals(randomQuorum);
  });

  it('should filter quorums by ttl', async () => {
    coreRpcClientMock.quorum.resolves({
      llmq_test_v17: [
        {
          '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c': {
            creationHeight: 100,
            minedBlockHash: '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
          },
        },
        {
          '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce': {
            creationHeight: 672,
            minedBlockHash: '251fae1f7fe89d38c4ce71781685c96291cc3852e9586c8eb6d5a71b73d6285c',
          },
        },
      ],
    });

    const result = await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
    expect(smlMock.getQuorum).to.have.been.calledOnceWithExactly(
      quorumType, '1af5ffbdb862a18b454106b6b99e30bb683fa5bab8278b19a6e91bf742405cce',
    );
    expect(result).to.equals(randomQuorum);
  });

  it('should choose from all quorums if filtered list is empty', async () => {
    smlMock.getQuorumsOfType.returns([
      {
        quorumHash: Buffer.alloc(1, 64).toString('hex'),
        validMembersCount: 10,
      },
      {
        quorumHash: Buffer.alloc(1, 32).toString('hex'),
        validMembersCount: 10,
      },
    ]);

    const result = await getRandomQuorum(smlMock, quorumType, Buffer.alloc(1), coreHeight);

    expect(smlMock.getQuorumsOfType).to.have.been.calledOnceWithExactly(quorumType);
    expect(smlMock.getQuorum).to.have.been.calledOnceWithExactly(
      quorumType, '20',
    );
    expect(result).to.equals(randomQuorum);
  });
});
