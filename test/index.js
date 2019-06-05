const dashcore = require('@dashevo/dashcore-lib');
const Blockchain = require('../lib/spvchain');
const utils = require('../lib/utils');
const merkleProofs = require('../lib/merkleproofs');

const {
  testnet, testnet2, testnet3, mainnet, badRawHeaders,
} = require('./data/rawHeaders');
const headers = require('./data/headers');
const merkleData = require('./data/merkleproofs');

let chain = null;

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
// Create scenarios where chain splits occur to form competing brances
// Difficult with current chain provided by chainmanager as this is actual hardcoded
// Dash testnet headers which requires significant CPU power to create forked chains from

describe('MerkleProofs', () => {
  it('should validate tx inclusion in merkleblock', () => {
    const merkleBlock = new dashcore.MerkleBlock(merkleData.merkleBlock);
    const validTx = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const invalidTx = `${validTx.substring(0, validTx.length - 1)}0`;

    merkleProofs.validateTxProofs(merkleBlock, [validTx]).should.equal(true);
    merkleProofs.validateTxProofs(merkleBlock, [invalidTx]).should.equal(false);
  });
});
