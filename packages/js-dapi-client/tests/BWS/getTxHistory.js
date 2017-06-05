const should = require('should');
require('../_before.js');

describe('BWS - getTxHistory', function() {
    it('should return the transaction history in short form', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 0, 0, false);
        res.should.be.a.Array();
        res[0].should.equal('348564ee88cb0eed8f26e4176b2a703b933a28588789f50cfc38d82883a941d2')
    });
    it('should return the transaction history in expanded form', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 0, 2, true);
        res[0].should.be.a.Object();
    });
    it('should have pagination', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 2, 10, false)
        res.should.be.a.Array();
        res.length.should.equal(8);

    });
});
