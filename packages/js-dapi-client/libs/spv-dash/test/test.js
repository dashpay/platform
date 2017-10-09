'use strict'

// create blockchain
const blockchain = require('../lib/spvchain');
const chainManager = require('./chainmanager');
const should = require('should');
const config = require('../config/config')

var initChainFromStorage = function() {
    //todo
}

var doConstuctChainThenValidateTest = function() {
    const filterAddr = 'yMSUh839et5ZF8bk3SXHA7NPbyDgigUbfG' //optional for bloomfilters

    //coinbase tx hash of block 3
    const validationTxHash = '1cc711129405a328c58d1948e748c3b8f3d610e66d9901db88c42c5247829658';

    //block 3 hash. Note if tx indexing is enabled (usally false) on full nodes this might be ommited 
    let validationBlockHash = null
    getHeaders(0)
        .then(headers => {
            validationBlockHash = headers[2]._getHash().toString('hex');
        })

    let localStoredFile = false;
    if (localStoredFile) {
        loadChainFromStorage();
    }
    else {
        let currHeight = chain.getChainHeight();

        getHeaders(currHeight + 1)
            .then(headers => {
                if (headers) {
                    chain._addHeaders(headers);
                    //Todo add headers until tip of blockchain
                    return true;
                }
                else {
                    //todo
                    return true;
                }
            })
            .then(success => {
                if (success) {
                    console.log(`Success: Added & validated blocks to SPV chain (building on genesis block)`)
                    return chain.getMerkleProof(validationBlockHash, validationTxHash, filterAddr, false)
                }
                else {
                    //todo                
                }

            })
            .then(isvalid => {
                if (isvalid) {
                    console.log(`${validationTxHash} is validated!`)
                    //todo: SDK.Explorer.API.getTx(validationTxHash) to update balances etc
                    //the resulting full tx string can be hashed again to make sure it equals validationTxHash
                }
                else {

                }
            })
            .catch(err => {
                console.log(` ${err}`)
            })
    }
}

var chain = null;
var headers = []
var initChain = function() {
    return new Promise((resolve, reject) => {
        chain = new blockchain(null, 'testnet')
        chain.on('ready', function() {
            resolve(true)
        }, this);
    });
}

describe('SPV-DASH (forks & re-orgs)', function() {

    it('should wait for chain to be ready', function() {
        return initChain()
            .then(res => {
                res.should.be.true();
            })

    });

    //save to disk to speedup
    it('should mine 5 test headers', function() {
        headers = chainManager.fetchHeaders();
        headers.length.should.equal(5);
    });

    it('should create a fork when adding header 0', function() {
        chain._addHeader(headers[0]);
        chain.forkedChains.length.should.equal(1);
    });

    it('should create a second fork when adding header 1', function() {
        chain._addHeader(headers[1]);
        chain.forkedChains.length.should.equal(2);
    });

    it('should have 4 total blocks on chain 2 (strongest chain) after adding headers 2,3 and 4', function() {
        chain._addHeaders(headers.slice(2, 5));
        chain.getChainHeight().should.equal(4); //genesis block + 1 matured block + 2 forked/pending blocks
    });

});

describe('SPV-DASH (merkle proofs)', function() {
    //Tests included in dapi-sdk
    //possibly add further tests here    
});





