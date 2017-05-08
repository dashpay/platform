require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlock', function() {
    it('should return the valid block from hash', async function(){
        let block = await SDK.Explorer.API.getBlock(validBlockHash);
        block.should.have.property('hash');
        block.should.have.property('size');
        block.should.have.property('height');
        block.should.have.property('version');
        block.should.have.property('merkleroot');
        block.should.have.property('tx');
        block.should.have.property('time');
        block.should.have.property('nonce');
        block.should.have.property('bits');
        block.should.have.property('difficulty');
        block.should.have.property('chainwork');
        block.should.have.property('confirmations');
        block.should.have.property('previousblockhash');
        block.should.have.property('reward');
        block.should.have.property('isMainChain');
        block.should.have.property('poolInfo');
        block.hash.should.equal(validBlockHash);
    });
    it('should return the valid block from height', async function(){
        let block = await SDK.Explorer.API.getBlock(validBlockHeight);
        block.should.have.property('hash');
        block.should.have.property('size');
        block.should.have.property('height');
        block.should.have.property('version');
        block.should.have.property('merkleroot');
        block.should.have.property('tx');
        block.should.have.property('time');
        block.should.have.property('nonce');
        block.should.have.property('bits');
        block.should.have.property('difficulty');
        block.should.have.property('chainwork');
        block.should.have.property('confirmations');
        block.should.have.property('previousblockhash');
        block.should.have.property('reward');
        block.should.have.property('isMainChain');
        block.should.have.property('poolInfo');
        block.hash.should.equal(validBlockHash);
    });
});