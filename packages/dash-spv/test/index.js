const { MerkleBlock, Transaction } = require('@dashevo/dashcore-lib');
const Blockchain = require('../lib/spvchain');
const utils = require('../lib/utils');
const merkleProofs = require('../lib/merkleproofs');
const consensus = require('../lib/consensus');
const {
  testnet, testnet2, testnet3, mainnet, badRawHeaders,
} = require('./data/rawHeaders');
const headers = require('./data/headers');
const merkleData = require('./data/merkleproofs');

let chain = null;
let merkleBlock = null;
let merkleBlock2 = null;

require('should');

describe('SPV-DASH (forks & re-orgs) deserialized headers', () => {
  before(() => {
    chain = new Blockchain('devnet');
  });

  it('should get 26 testnet headers', () => {
    headers.length.should.equal(26);
  });

  it('should contain 1 branch when chain is initialised with genesis block', () => {
    chain.getAllBranches().length.should.equal(1);
  });

  it('should contain genesis hash', () => {
    chain.getTipHash().should.equal('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c');
    chain.getLongestChain().length.should.equal(1);
  });

  it('should still contain a branch of 1 when first header is added', () => {
    chain.addHeader(headers[0]);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should discard adding of duplicate block', () => {
    chain.addHeader(headers[0]);
    chain.getOrphans().length.should.equal(0);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should create 1 orphan', () => {
    chain.addHeader(headers[2]);
    chain.getOrphans().length.should.equal(1);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should connect the orphan by adding its parent', () => {
    chain.addHeader(headers[1]);
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(4);
  });

  it('should add remaining test headers', () => {
    chain.addHeaders(headers.slice(3, 24));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(25);
  });

  it('not add an invalid header', () => {
    chain.addHeader(headers[25]);
    chain.getLongestChain().length.should.equal(25);
  });

  it('should throw an error if some of the headers is invalid', (done) => {
    try {
      chain.addHeaders([headers[25], headers[10]]);
      done(new Error('SPV chain failed to throw an error on invalid block'));
    } catch (e) {
      e.message.should.equal('Some headers are invalid');
      done();
    }
  });
});

describe('SPV-DASH (forks & re-orgs) serialized raw headers for mainnet', () => {
  before(() => {
    chain = new Blockchain('mainnet', 10000, utils.normalizeHeader(mainnet[0]));
  });

  it('should get 2000 mainnet headers', () => {
    mainnet.length.should.equal(2000);
  });

  it('should contain 1 branch when chain is initialised with genesis block', () => {
    chain.getAllBranches().length.should.equal(1);
  });

  it('should contain start hash', () => {
    chain.getTipHash().should.equal('000000000000002b8a8363ce87b4c48087ff8a997a8102097102bed001ebc531');
    chain.getLongestChain().length.should.equal(1);
  });

  it('should still contain a branch of 1 when first header is added', () => {
    chain.addHeader(mainnet[1]);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should discard addding of duplicate block', () => {
    chain.addHeader(mainnet[1]);
    chain.getOrphans().length.should.equal(0);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should create 1 orphan', () => {
    chain.addHeader(mainnet[3]);
    chain.getOrphans().length.should.equal(1);
    chain.getLongestChain().length.should.equal(2);
  });

  it('should connect the orphan by adding its parent', () => {
    chain.addHeader(mainnet[2]);
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(4);
  });
});

describe('SPV-DASH (addHeaders) add many headers for testnet', () => {
  before(() => {
    chain = new Blockchain('testnet', 10000, utils.normalizeHeader(testnet[0]));
  });

  it('should add the 1st 250 testnet headers', () => {
    chain.addHeaders(testnet.slice(1, 250));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(250);
  });

  it('should add the next 250 (250 - 500) testnet headers', () => {
    chain.addHeaders(testnet.slice(250, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should add the 1st 250 testnet2 headers', () => {
    chain = new Blockchain('testnet', 10000, utils.normalizeHeader(testnet2[0]));
    chain.addHeaders(testnet2.slice(1, 250));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(250);
  });

  it('should add the next 250 (250 - 500) testnet2 headers', () => {
    chain.addHeaders(testnet2.slice(250, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should add the 1st 250 testnet3 headers', () => {
    chain = new Blockchain('testnet', 10000, utils.normalizeHeader(testnet3[0]));
    chain.addHeaders(testnet3.slice(1, 250));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(250);
  });

  it('should add the next 250 (250 - 500) testnet3 headers', () => {
    chain.addHeaders(testnet3.slice(250, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should not add an invalid header', () => {
    chain.addHeader(testnet[499]);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should orphan and not add invalid but consistent headers', () => {
    chain.addHeaders([badRawHeaders[0], badRawHeaders[1]]);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should throw an error if some of the headers are inconsistent', (done) => {
    try {
      chain.addHeaders([badRawHeaders[0], badRawHeaders[2]]);
      done(new Error('SPV chain failed to throw an error on invalid block'));
    } catch (e) {
      e.message.should.equal('Some headers are invalid');
      done();
    }
  });
});

describe('SPV-DASH (addHeaders) add testnet headers out of order', () => {
  before(() => {
    chain = new Blockchain('testnet', 10000, utils.normalizeHeader(testnet[0]));
  });

  it('should add the 1st 100 testnet headers', () => {
    chain.addHeaders(testnet.slice(1, 100));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(0);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should orphan testnet headers 200 - 300', () => {
    chain.addHeaders(testnet.slice(200, 300));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should orphan testnet headers 400 - 500', () => {
    chain.addHeaders(testnet.slice(400, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(2);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should reconnect orphaned chunks (testnet headers 1 - 100 and 200 - 300)', () => {
    chain.addHeaders(testnet.slice(100, 200));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(300);
  });

  it('should reconnect orphaned chunks (testnet headers 1 - 300 and 400 - 500)', () => {
    chain.addHeaders(testnet.slice(300, 400));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(0);
    chain.getLongestChain().length.should.equal(500);
  });
});

describe('SPV-DASH (addHeaders) add many headers for mainnet', () => {
  before(() => {
    chain = new Blockchain('mainnet', 10000, utils.normalizeHeader(mainnet[0]));
  });

  it('should add the 1st 500 mainnet headers', () => {
    chain.addHeaders(mainnet.slice(1, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(500);
  });

  it('should add the next 500 (500 - 1000) mainnet headers', () => {
    chain.addHeaders(mainnet.slice(500, 1000));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(1000);
  });

  it('should add the next 500 (1000 - 1500) mainnet headers', () => {
    chain.addHeaders(mainnet.slice(1000, 1500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getLongestChain().length.should.equal(1500);
  });

  it('should not add an invalid header', () => {
    chain.addHeader(mainnet[499]);
    chain.getLongestChain().length.should.equal(1500);
  });

  it('should orphan and not add invalid but consistent headers', () => {
    chain.addHeaders([badRawHeaders[0], badRawHeaders[1]]);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(1500);
  });

  it('should throw an error if some of the headers are inconsistent', (done) => {
    try {
      chain.addHeaders([badRawHeaders[0], badRawHeaders[2]]);
      done(new Error('SPV chain failed to throw an error on invalid block'));
    } catch (e) {
      e.message.should.equal('Some headers are invalid');
      done();
    }
  });
});

describe('SPV-DASH (addHeaders) add mainnet headers out of order', () => {
  before(() => {
    chain = new Blockchain('mainnet', 10000, utils.normalizeHeader(mainnet[0]));
  });

  it('should add the 1st 100 mainnet headers', () => {
    chain.addHeaders(mainnet.slice(1, 100));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(0);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should orphan mainnet headers 200 - 300', () => {
    chain.addHeaders(mainnet.slice(200, 300));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should orphan mainnet headers 400 - 500', () => {
    chain.addHeaders(mainnet.slice(400, 500));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(2);
    chain.getLongestChain().length.should.equal(100);
  });

  it('should reconnect orphaned chunks (mainnet headers 1 - 100 and 200 - 300)', () => {
    chain.addHeaders(mainnet.slice(100, 200));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(1);
    chain.getLongestChain().length.should.equal(300);
  });

  it('should reconnect orphaned chunks (mainnet headers 1 - 300 and 400 - 500)', () => {
    chain.addHeaders(mainnet.slice(300, 400));
    chain.getOrphans().length.should.equal(0);
    chain.getAllBranches().length.should.equal(1);
    chain.getOrphanChunks().length.should.equal(0);
    chain.getLongestChain().length.should.equal(500);
  });
});

let genesisHash = null;
describe('Blockstore', () => {
  before(() => {
    chain = new Blockchain('testnet', 10);
    genesisHash = chain.getTipHash();
  });

  it('should add 9 headers', (done) => {
    chain.addHeaders(headers.slice(0, 9));
    chain.getLongestChain().length.should.equal(10);
    chain.getHeader(genesisHash)
      .then((header) => {
        header.hash.should.equal(genesisHash);
        header.should.have.property('children');
        done();
      });
  });

  it('should move 1 block to the blockstore', (done) => {
    chain.addHeaders(headers.slice(9, 10));
    chain.getLongestChain().length.should.equal(10);
    chain.getHeader(genesisHash)
      .then((header) => {
        header.hash.should.equal(genesisHash);
        header.should.not.have.property('children');
        done();
      });
  });
});

// TODO:
// Create scenarios where chain splits occur to form competing branches
// Difficult with current chain provided by chainmanager as this is actually hardcoded
// Dash testnet headers which requires significant CPU power to create forked chains from

describe('MerkleProofs', () => {
  it('should validate tx inclusion in merkleblock', () => {
    merkleBlock = new MerkleBlock(merkleData.merkleBlock);
    const validTx = 'c5e85bfa5c0279433f8b4d33fe67f726d7fef1625506cb93554d0170e2bfaf45';
    const invalidTx = `${validTx.substring(0, validTx.length - 1)}0`;
    merkleProofs.validateTxProofs(merkleBlock, [validTx]).should.equal(true);
    merkleProofs.validateTxProofs(merkleBlock, [invalidTx]).should.equal(false);
  });
});

describe('Transaction validation', () => {
  before(() => {
    chain = new Blockchain('testnet', 10000, utils.normalizeHeader(testnet[0]));
    chain.addHeaders(testnet.slice(1, 500));
  });

  beforeEach(() => {
    merkleBlock = new MerkleBlock(Buffer.from(merkleData.rawMerkleBlock, 'hex'));
    merkleBlock2 = new MerkleBlock(Buffer.from(merkleData.rawMerkleBlock2, 'hex'));
  });

  it('should throw an error if wrong type is passed', async () => {
    const validTx = '7262476912a96b9a6226cfa3a8f231ba3e2b1f75c396e88367e532c79c43c95b';
    const invalid = Buffer.from(validTx);
    try {
      await consensus.areValidTransactions(invalid, merkleBlock, chain);
      throw new Error('Transaction validation failed to throw an error');
    } catch (e) {
      e.message.should.equal('Please check that transactions parameter is a non-empty array');
    }
  });

  it('should throw an error if empty transactions array', async () => {
    const transactions = [];
    try {
      await consensus.areValidTransactions(transactions, merkleBlock, chain);
      throw new Error('Transaction validation failed to throw an error');
    } catch (e) {
      e.message.should.equal('Please check that transactions parameter is a non-empty array');
    }
  });

  it('should not validate an array of raw transactions for a merkleblock that was generated with a filter containing only one of them', async () => {
    const validTx = new Transaction('020000000100de7192338db34fe9bb25f34122893d94f3b43bd4c881e37924c8e95a068cc8000000006b483045022100b185b4b86b613e3ffc796db90f95dc88f82561c50ba49fa610d8090f61f38ff002201473466bddee2672ed0dba75b81c07bdade734441e005c8c7fdf12a039ff9312012102bdfedbfe6ea05de8094d18442e08c98ebd695acf489f1bdf68fe1e3aff6f488effffffff010000000000000000016a00000000');
    const validTx2 = new Transaction('0200000001ccc68ff58b7b02247f3e05440ab7fc7c8c599453de4a49e35393981890a1e984010000006b483045022100e141365c4916fa09d03aac58f215b926b777a3acec918a6becdbb03d59d28d9f02204edc1ce68d596e28e55f114c67a270302d488707e754af1261fdfc043891651c012102d5b7c0dfb2fd9591a4a98555ce806e17842f401979f2e0cc0689c91d6ca9ef87feffffff025d4c0000000000001976a9146d14b25994e4036d70eeafd4a706640337db5a5e88ac409c0000000000001976a9148b4a9da5a46c7b89b26b649bac8e34e7aa5aa63188acc2260000');
    const transactions = [];
    transactions.push(validTx);
    transactions.push(validTx2);
    const result = await consensus.areValidTransactions(transactions, merkleBlock, chain);
    result.should.equal(false);
  });

  it('should validate an array of raw transactions for a merkleblock that was generated with a filter containing both of them', async () => {
    const validTx = new Transaction('020000000100de7192338db34fe9bb25f34122893d94f3b43bd4c881e37924c8e95a068cc8000000006b483045022100b185b4b86b613e3ffc796db90f95dc88f82561c50ba49fa610d8090f61f38ff002201473466bddee2672ed0dba75b81c07bdade734441e005c8c7fdf12a039ff9312012102bdfedbfe6ea05de8094d18442e08c98ebd695acf489f1bdf68fe1e3aff6f488effffffff010000000000000000016a00000000');
    const validTx2 = new Transaction('0200000001ccc68ff58b7b02247f3e05440ab7fc7c8c599453de4a49e35393981890a1e984010000006b483045022100e141365c4916fa09d03aac58f215b926b777a3acec918a6becdbb03d59d28d9f02204edc1ce68d596e28e55f114c67a270302d488707e754af1261fdfc043891651c012102d5b7c0dfb2fd9591a4a98555ce806e17842f401979f2e0cc0689c91d6ca9ef87feffffff025d4c0000000000001976a9146d14b25994e4036d70eeafd4a706640337db5a5e88ac409c0000000000001976a9148b4a9da5a46c7b89b26b649bac8e34e7aa5aa63188acc2260000');
    const transactions = [];
    transactions.push(validTx);
    transactions.push(validTx2);
    const result = await consensus.areValidTransactions(transactions, merkleBlock2, chain);
    result.should.equal(true);
  });

  it('should not validate an array of transactions hashes for a merkleblock that was generated with a filter containing only one of them', async () => {
    const validTxHash = '7262476912a96b9a6226cfa3a8f231ba3e2b1f75c396e88367e532c79c43c95b';
    const validTxHash2 = '3f3517ee8fa95621fe8abdd81c1e0dfb50e21dd4c5a3c01eee2c47cf664821b6';
    const transactions = [];
    transactions.push(validTxHash);
    transactions.push(validTxHash2);
    const result = await consensus.areValidTransactions(transactions, merkleBlock, chain);
    result.should.equal(false);
  });

  it('should validate an array of transactions hashes for a merkleblock that was generated with a filter containing both of them', async () => {
    const validTxHash = '7262476912a96b9a6226cfa3a8f231ba3e2b1f75c396e88367e532c79c43c95b';
    const validTxHash2 = '3f3517ee8fa95621fe8abdd81c1e0dfb50e21dd4c5a3c01eee2c47cf664821b6';
    const transactions = [];
    transactions.push(validTxHash);
    transactions.push(validTxHash2);
    const result = await consensus.areValidTransactions(transactions, merkleBlock2, chain);
    result.should.equal(true);
  });
});
