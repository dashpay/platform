require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockSize', function() {
    it('should return the valid getBlockSize from hash', async function(){
        let blockSize = await SDK.Explorer.API.getBlockSize(validBlockHash);
        blockSize.should.equal(1566);
    });
    it('should return the valid getBlockSize from height', async function(){
        let blockSize = await SDK.Explorer.API.getBlockSize(validBlockHeight);
        blockSize.should.equal(1566);
    });
});