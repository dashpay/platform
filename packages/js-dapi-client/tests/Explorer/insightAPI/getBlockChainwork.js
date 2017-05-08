require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockChainwork', function() {
    it('should return the valid chainwork from hash', async function(){
        let chainwork = await SDK.Explorer.API.getBlockChainwork(validBlockHash);
        chainwork.should.equal('0000000000000000000000000000000000000000000000000000567c1242e904');
    });
    it('should return the valid chainwork from height', async function(){
        let chainwork = await SDK.Explorer.API.getBlockChainwork(validBlockHeight);
        chainwork.should.equal('0000000000000000000000000000000000000000000000000000567c1242e904');
    });
});