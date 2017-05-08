require('../../_before.js');
const should = require('should');

describe('Insight-API - getLastBlockHeight', function() {
    it('should return the valid block height', async function(){
        let blockHeight = await SDK.Explorer.API.getLastBlockHeight();
        blockHeight.should.be.type('number');//TODO : We want to use bitcore to verify the validity of the address aswell
    });
});