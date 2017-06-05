const should = require('should');
require('../_before.js');

describe('BWS - broadcastRawTx', function() {
    it('should return a boolean', async function(){
        let opts = 1;
        let network = 1;
        let rawTx = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff13033911030e2f5032506f6f6c2d74444153482fffffffff0479e36542000000001976a914f0adf747fe902643c66eb6508305ba2e1564567a88ac40230e43000000001976a914f9ee3a27ef832846cf4ad40fe95351effe4a485d88acc73fa800000000004341047559d13c3f81b1fadbd8dd03e4b5a1c73b05e2b980e00d467aa9440b29c7de23664dde6428d75cafed22ae4f0d302e26c5c5a5dd4d3e1b796d7281bdc9430f35ac00000000000000002a6a283662876fa09d54098cc66c0a041667270a582b0ea19428ed975b5b5dfb3bca79000000000200000000000000";
        return await SDK.BWS.broadcastRawTx(opts,network,rawTx)
                            .then(res=> thow,
                                  err=>{
                                    err.should.be.a.String()
                                    err.split(' ').should.containDeep(['error'])
                                  })
    });
});
