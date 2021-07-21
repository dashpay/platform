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
    rootHash = Buffer.from('a9220603765eb43567e1ee316409e107e131c97daa7c488463998032458333aa', 'hex');

    leafOneMock = {
      getIndex() {
        return 0;
      },
      getHash() {
        return hashFunction(leafOneRootHash);
      },
      getProof() {
        return Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex');
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

    it('should return empty buffer if leafHashes consist of empty buffers', () => {
      leafOneMock.getHash = () => Buffer.alloc(20);
      leafTwoMock.getHash = () => Buffer.alloc(20);

      rootTree = new RootTree([leafOneMock, leafTwoMock]);

      const actualRootHash = rootTree.getRootHash();

      expect(actualRootHash).to.deep.equal(Buffer.alloc(0));
    });
  });

  describe('#getProof', () => {
    it('should return a proof for the first leaf', () => {
      const proof = rootTree.getProof(leafOneMock);

      expect(proof).to.deep.equal(
        Buffer.from('0100000001e4d60b4531b114100aab5f9907d1718c613e603482a15bee8ccda17e5c9bb3ea0101', 'hex'),
      );
    });

    it('should return a proof for the second leaf', () => {
      const proof = rootTree.getProof(leafTwoMock);

      expect(proof).to.deep.equal(
        Buffer.from('01000000019515049071ed913149a80d3bb7891fcd4c6c1e3d14ad878939a80f9b9a91e08c0100', 'hex'),
      );
    });
  });

  describe('#getFullProof', () => {
    it('should return a full proof', () => {
      const leafKeys = [
        Buffer.from([1]),
      ];

      const fullProof = rootTree.getFullProof(leafOneMock, leafKeys);

      expect(fullProof).to.be.deep.equal({
        rootTreeProof: Buffer.from('0100000001e4d60b4531b114100aab5f9907d1718c613e603482a15bee8ccda17e5c9bb3ea0101', 'hex'),
        storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
      });
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
