require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getHeightFromHash', function() {
    it('should return the valid Height from hash', function() {
        return SDK.Explorer.API.getHeightFromHash(validBlockHash)
            .then(height => height.should.equal(validBlockHeight));
    });
});