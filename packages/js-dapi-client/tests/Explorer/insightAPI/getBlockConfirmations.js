require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockConfirmations', async function() {
    it('should return the valid confirmations from hash', async function(){
        let getLastBlock = await SDK.Explorer.API.getLastBlock();
        let expectedDiff = (getLastBlock.height-validBlockHeight)+1;
        let confirmations = await SDK.Explorer.API.getBlockConfirmations(validBlockHash);
        confirmations.should.equal(expectedDiff);
    });
    it('should return the valid confirmations from height', async function(){
        let getLastBlock = await SDK.Explorer.API.getLastBlock();
        let expectedDiff = (getLastBlock.height-validBlockHeight)+1;
        let confirmations = await SDK.Explorer.API.getBlockConfirmations(validBlockHeight);
        confirmations.should.equal(expectedDiff);
    });
});