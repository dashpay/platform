require('../../_before.js');
const should = require('should');

describe('Insight-API - getLastBlock', function() {
    it('should return the valid block', function() {
        return SDK.Explorer.API.getLastBlock()
            .then(block => {
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
                // block.should.have.property('reward');
                block.should.have.property('isMainChain');
                block.should.have.property('poolInfo');
            })
    });
});