require('../../_before.js');
const should = require('should');

describe('Insight-API - getLastBlockHeight', function() {
    it('should return the valid block height', function() {
        return SDK.Explorer.API.getLastBlockHeight()
            .then(blockHeight => blockHeight.should.be.type('number'))
        //TODO : We want to use bitcore to verify the validity of the address aswell
    });
});