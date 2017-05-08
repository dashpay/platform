require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockMerkleRoot', function() {
    it('should return the valid getBlockMerkleRoot from hash', async function(){
        let merkleRoot = await SDK.Explorer.API.getBlockMerkleRoot(validBlockHash);
        merkleRoot.should.equal('e270a45c438fb8befead6b3c9e88201b0aa96e5f43920fb78fdb8dbb3e433e04');
    });
    it('should return the valid confirmations from height', async function(){
        let merkleRoot = await SDK.Explorer.API.getBlockMerkleRoot(validBlockHeight);
        merkleRoot.should.equal('e270a45c438fb8befead6b3c9e88201b0aa96e5f43920fb78fdb8dbb3e433e04');
    });
});