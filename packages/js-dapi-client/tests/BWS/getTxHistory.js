const should = require('should');
require('../_before.js');

describe('BWS - getTxHistory', function() {
    it('should return the transaction history in short form', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 0, 0, false);
        res.should.be.a.Array();
        res[0].should.equal('f226064462336e11d17acca6be0708d56175b38f769677766ca0eba2f5d08f93')
    });
    it('should return the transaction history in expanded form', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 0, 2, true, (err, x)=> x);
        res[0].should.be.a.Object();
    });
    it('should have pagination', async function(){
        let res = await SDK.BWS.getTxHistory({addr:"yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4"}, 2, 10, false)
        res.should.be.a.Array();
        res.length.should.equal(8);

    });
});