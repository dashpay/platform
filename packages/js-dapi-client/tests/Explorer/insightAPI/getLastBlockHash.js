require('../../_before.js');
const should = require('should');

describe('Insight-API - getLastBlockHash', function() {
    it('should return the valid block hash', function(){
        return SDK.Explorer.API.getLastBlockHash()
        .then(blockHash => blockHash.should.be.type('string'))
    });
});