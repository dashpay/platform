const { MerkleProof } = require('js-merkle');
const RootTree = require('../../../lib/rootTree/RootTree');

const { init: initHashFunction, hashFunction } = require('../../../lib/rootTree/hashFunction');

const InvalidLeafIndexError = require('../../../lib/rootTree/errors/InvalidLeafIndexError');

const ROOT_TREE_LEAVES_COUNT = 6;

describe('RootTree', () => {
  let leafMocks;
  let leafOneMock;
  let leafTwoMock;
  let leafHashes;
  let rootTree;
  let rootHash;

  before(async () => {
    await initHashFunction();
  });

  beforeEach(() => {
    leafMocks = [];
    leafHashes = [];
    for (let i = 0; i < 6; i++) {
      const leafHash = Buffer.alloc(32, i);
      leafHashes.push(leafHash);
      leafMocks.push({
        getIndex() {
          return i;
        },
        getHash() {
          return hashFunction(leafHash);
        },
        getProof() {
          return Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex');
        },
      });
    }

    rootHash = Buffer.from('b802d7671d81952e67ddb07401368103e2d3f7c9dbbbe6d3ec45dafda13d0d15', 'hex');

    [leafOneMock, leafTwoMock] = leafMocks;

    rootTree = new RootTree(leafMocks);
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
      const proof = rootTree.getProof([leafOneMock]);

      expect(proof.length).to.be.equal(32 * 3);
      expect(proof).to.deep.equal(
        Buffer.from('9515049071ed913149a80d3bb7891fcd4c6c1e3d14ad878939a80f9b9a91e08c1c5ed0487eb404ff32745cf8f0ff6fc0c37c1b01ffaf21e7855accca7dd4cd0e057e710f2441e229de9240f2153266c8a892831d92b8ba3190adae06dddc39db', 'hex'),
      );
    });

    it('should return a proof for the second leaf', () => {
      const proof = rootTree.getProof([leafTwoMock]);

      expect(proof.length).to.be.equal(32 * 3);
      expect(proof).to.deep.equal(
        Buffer.from('2ada83c1819a5372dae1238fc1ded123c8104fdaa15862aaee69428a1820fcda1c5ed0487eb404ff32745cf8f0ff6fc0c37c1b01ffaf21e7855accca7dd4cd0e057e710f2441e229de9240f2153266c8a892831d92b8ba3190adae06dddc39db', 'hex'),
      );
    });
  });

  describe('#getFullProofForOneLeaf', () => {
    it('should return a full proof', () => {
      const leafKeys = [
        Buffer.from([1]),
      ];

      const fullProof = rootTree.getFullProofForOneLeaf(leafOneMock, leafKeys);

      expect(fullProof).to.be.deep.equal({
        rootTreeProof: Buffer.from('9515049071ed913149a80d3bb7891fcd4c6c1e3d14ad878939a80f9b9a91e08c1c5ed0487eb404ff32745cf8f0ff6fc0c37c1b01ffaf21e7855accca7dd4cd0e057e710f2441e229de9240f2153266c8a892831d92b8ba3190adae06dddc39db', 'hex'),
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

  describe('#verification', () => {
    it('should verify proof', () => {
      const proofBuffer = rootTree.getProof([leafMocks[4], leafMocks[5]]);

      const proof = MerkleProof.fromBuffer(proofBuffer, hashFunction);

      const result = proof.verify(
        rootTree.tree.getRoot(),
        [
          leafMocks[4].getIndex(),
          leafMocks[5].getIndex(),
        ],
        [
          leafMocks[4].getHash(),
          leafMocks[5].getHash(),
        ],
        ROOT_TREE_LEAVES_COUNT,
      );

      expect(result).to.be.true();
    });
  });
});
