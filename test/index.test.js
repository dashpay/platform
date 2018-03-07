const Blockchain = require('../lib/spvchain');
const chainManager = require('./chainmanager');

let chain = null;
let headers = [];
require('should');

// function doConstuctChainThenValidateTest() {
//   const filterAddr = 'yMSUh839et5ZF8bk3SXHA7NPbyDgigUbfG'; // optional for bloomfilters

//   // coinbase tx hash of block 3
//   const validationTxHash = '1cc711129405a328c58d1948e748c3b8f3d610e66d9901db88c42c5247829658';

//   // block 3 hash. Note if tx indexing is enabled (usally false) on full nodes this might be
// ommited
//   let validationBlockHash = null;
//   getHeaders(0)
//     .then((headers) => {
//       validationBlockHash = headers[2]._getHash().toString('hex');
//     });

//   const localStoredFile = false;
//   if (localStoredFile) {
//     loadChainFromStorage();
//   } else {
//     const currHeight = chain.getChainHeight();

//     getHeaders(currHeight + 1)
//       .then((headers) => {
//         if (headers) {
//           chain._addHeaders(headers);
//           // Todo add headers until tip of blockchain
//           return true;
//         }
//         // todo
//         return true;
//       })
//       .then((success) => {
//         if (success) {
//           console.log(
//             'Success: Added & validated blocks to SPV chain (building on genesis block)');
//           return chain.getMerkleProof(validationBlockHash, validationTxHash, filterAddr, false);
//         }
//         // todo
//       })
//       .then((isvalid) => {
//         if (isvalid) {
//           console.log(`${validationTxHash} is validated!`);
//           // todo: SDK.Explorer.API.getTx(validationTxHash) to update balances etc
//           // the resulting full tx string can be hashed again to make sure it equals
//           validationTxHash
//         } else {

//         }
//       })
//       .catch((err) => {
//         console.log(` ${err}`);
//       });
//   }
// }


describe('SPV-DASH (forks & re-orgs)', () => {
  before(() => {
    headers = chainManager.fetchHeaders();
    chain = new Blockchain(null, 'testnet');
  });

  it('should get 25 testnet headers', () => {
    headers.length.should.equal(25);
  });

  it('should contain no forks when chain is initialised with genesis block', () => {
    chain.forkedChains.length.should.equal(0);
  });

  it('should contain genesis hash', () => {
    chain.getTipHash().should.equal('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c');
  });

  it('should contain a fork of 1 when first header is added', () => {
    chain.addHeader(headers[0]);
    chain.forkedChains.length.should.equal(1);
  });

  it('should contain correct tip and height when remaining headers [1..24] is added', () => {
    headers.shift();
    chain.addHeaders(headers);
    chain.getChainHeight().should.equal(26);
  });
});

describe('SPV-DASH (merkle proofs)', () => {
  // Tests included in dapi-sdk
  // possibly add further tests here
});

