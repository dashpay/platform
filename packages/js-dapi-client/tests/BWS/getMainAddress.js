const should = require('should');
require('../_before.js');


describe('BWS - getMainAddress', function() {
    it('should addresses in blocks of 20 with valid balance, at least 1', async function(){
        let wd = "inflict about smart zoo ethics ignore retire expand peasant draft sock raven";
        let res = await SDK.BWS.getMainAddress({},{},10,{},{},wd)
        res.should.be.a.Array();
        res[0].should.equal('yih9qioDMT5e1pZs7idndepvhsQLf1wkAa')
    });
});
