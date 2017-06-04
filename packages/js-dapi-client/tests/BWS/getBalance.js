sould = require('should');
require('../_before.js');

let addrs =  ['yb21342iADyqAotjwcn4imqjvAcdYhnzeH', 'yUGETMg58sQd7mTHEZJKqaEYvvXc7udrsh']

describe('BWS - getBalance', function() {
    it('should return the fee as a number', async function(){
        let res = await SDK.BWS.getBalance(1,(err, x)=>x,addrs[0]);
        res.should.be.a.Number();
        res.should.be.aboveOrEqual(0);
        ;
    });
});