require('../../_before.js');
const should = require('should');

describe('Insight-API - getLastBlockHash', function() {
    it('should return the valid block hash', async function(){
        let blockHash = await SDK.Explorer.API.getLastBlockHash();
        blockHash.should.be.type('string');//TODO : We want to use bitcore to verify the validity of the address aswell
    });
});