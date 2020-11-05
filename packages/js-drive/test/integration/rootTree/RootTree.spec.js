const RootTree = require('../../../lib/rootTree/RootTree');

const hashFunction = require('../../../lib/rootTree/hashFunction');

const InvalidLeafIndexError = require('../../../lib/rootTree/errors/InvalidLeafIndexError');

describe('RootTree', () => {
  let leafOneMock;
  let leafTwoMock;
  let rootTree;
  let rootHash;

  beforeEach(() => {
    const leafOneRootHash = Buffer.alloc(32).fill(1);
    const leafTwoRootHash = Buffer.alloc(32).fill(2);
    rootHash = Buffer.from('e1cea2a83f3686b0183922ba4baf0e0d2f3e0622', 'hex');

    leafOneMock = {
      getIndex() {
        return 0;
      },
      getHash() {
        return hashFunction(leafOneRootHash);
      },
    };

    leafTwoMock = {
      getIndex() {
        return 1;
      },
      getHash() {
        return hashFunction(leafTwoRootHash);
      },
    };

    rootTree = new RootTree([leafOneMock, leafTwoMock]);
  });

  describe('#constructor', () => {
    it('should throw an error if a leaf index in not corresponding to leaves param order', () => {
      expect(() => {
        // eslint-disable-next-line no-new
        new RootTree([leafTwoMock, leafOneMock]);
      }).to.throw(InvalidLeafIndexError);
    });
  });

  describe('#getRootHash', () => {
    it('should return merkle root calculated with specified leaves', () => {
      const actualRootHash = rootTree.getRootHash();

      expect(actualRootHash).to.deep.equal(rootHash);
    });
  });

  describe('#getProof', () => {
    it('should return a proof for the first leaf', () => {
      const proof = rootTree.getProof(leafOneMock);

      expect(proof).to.deep.equal([
        {
          position: 'right',
          data: Buffer.from('f0faf5f55674905a68eba1be2f946e667c1cb501', 'hex'),
        },
      ]);
    });

    it('should return a proof for the second leaf', () => {
      const proof = rootTree.getProof(leafTwoMock);

      expect(proof).to.deep.equal([
        {
          position: 'left',
          data: Buffer.from('fa5c47912cc22dce628071b48d2386bd511656e3', 'hex'),
        },
      ]);
    });
  });

  describe('#rebuild', () => {
    it('should rebuild root tree with updated leaf hashes', () => {
      leafOneMock.getHash = () => Buffer.alloc(32).fill(3);

      let actualRootHash = rootTree.getRootHash();

      expect(actualRootHash).to.deep.equal(rootHash);

      rootTree.rebuild();

      actualRootHash = rootTree.getRootHash();

      expect(actualRootHash).to.not.deep.equal(rootHash);
    });
  });
});
