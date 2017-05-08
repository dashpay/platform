require('../../_before.js');
const should = require('should');

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
describe('Insight-API - getBlockTransactions', function() {
    it('should return the valid getBlockTransactions from hash', async function(){
        let blockTransactions = await SDK.Explorer.API.getBlockTransactions(validBlockHash);
        blockTransactions.should.deepEqual(txArr);
    });
    it('should return the valid getBlockTransactions from height', async function(){
        let blockTransactions = await SDK.Explorer.API.getBlockTransactions(validBlockHeight);
        blockTransactions.should.deepEqual(txArr);
    });
});

let txArr = [
        'd6cb7a6756a2a9649d1ea587490516a6e3e3f3414d595281c889097a8ca44d23',
        '01235ae0b3f93656ebf062eb4f80a4af0cb50663993a48b84a8224e50177f976'
    ];