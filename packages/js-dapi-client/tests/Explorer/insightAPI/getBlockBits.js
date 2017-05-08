const should = require('should');
require('../../_before.js');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockBits', function() {
    it('should return the valid block bits from hash', async function(){
        let bits = await SDK.Explorer.API.getBlockBits(validBlockHash);
        bits.should.equal('1d01000d');
    });
    it('should return the valid block from height', async function(){
        let bits = await SDK.Explorer.API.getBlockBits(validBlockHeight);
        bits.should.equal('1d01000d');
    });
});