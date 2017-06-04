require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockConfirmations', function() {
    it('should return the valid confirmations from hash', function() {
        return Promise.all([SDK.Explorer.API.getLastBlock(), SDK.Explorer.API.getBlockConfirmations(validBlockHash)])
            .then(([lastBlock, confirmations]) => {
                let expectedDiff = (lastBlock.height - validBlockHeight) + 1;
                confirmations.should.equal(expectedDiff);
            })

    });
    it('should return the valid confirmations from height', function() {
        return Promise.all([SDK.Explorer.API.getLastBlock(), SDK.Explorer.API.getBlockConfirmations(validBlockHash)])
            .then(([lastBlock, confirmations]) => {
                let expectedDiff = (lastBlock.height - validBlockHeight) + 1;
                confirmations.should.equal(expectedDiff);
            })
    })

});