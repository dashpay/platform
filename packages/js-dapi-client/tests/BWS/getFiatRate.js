const should = require('should');
require('../_before.js');


describe('BWS - get fiat rate', function() {
    it('should return the rate', async function(){
        let res= await SDK.BWS.getFiatRate({},{},{},{},(err, x)=> x); //other params
        res.should.be.a.Object();
        res.should.have.property('rate');
        res.rate.should.equal(120);

    });
});