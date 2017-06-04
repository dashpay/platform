const should = require('should');
require('../_before.js');

describe('BWS - getFeeLevels', function() {
    it('should return the fee as a number', function(){
        return SDK.BWS.getFeeLevels('live',(err, x)=>x)
            .then(res => {
                res.should.be.a.Number();
            });
    });
});