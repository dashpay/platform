require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockVersion', function() {
    it('should return the valid getBlockVersion from hash', async function(){
        let version = await SDK.Explorer.API.getBlockVersion(validBlockHash);
        version.should.equal(536870912);
    });
    it('should return the valid getBlockVersion from height', async function(){
        let version = await SDK.Explorer.API.getBlockVersion(validBlockHeight);
        version.should.equal(536870912);
    });
});