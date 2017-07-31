'use strict'
const should = require('should');
const assert = require('assert');
const Node = require('../../lib/node/node');
const mocks = require('../../lib/mocks/mocks');

describe('Network - Mempool', function() {

    it('should verify ipfs deamon is running', function(done) {
        //todo
        done()
    })

    it('should sync a value from a VALID masternode on the list of masternodes', function(done) {
        const key = 'mn_valid_sync';
        const value = new Date().getTime();
        var nodes = [];

        mocks.mnList.map(mn => {
            let parms = {
                pubKey: mn.publicAdr,
                privKey: mn.privKey
            }
            nodes.push(new Node(parms));
        })

        nodes[0].addMemPoolData(nodes[0].config.privKey, nodes[0].config.pubKey, value, key);

        setTimeout(() => {

            nodes.filter(n => {
                let data = n.getMemPoolData(key)
                return data && data.value == value;
            }).length.should.equal(mocks.mnList.length);
            done();

        }, 1000)
    })

    it('should sync a value from a VALID masternode on the list of masternodes', function(done) {
        const key = 'mn_invalid_sync';
        const value = new Date().getTime();
        var nodes = [];

        mocks.mnList.map(mn => {
            let parms = {
                pubKey: mn.publicAdr,
                privKey: mn.privKey
            }
            nodes.push(new Node(parms));
        })

        //change MN privkey to valid key but not in the mnList to simulate invalid MN
        nodes[0].config.privKey = 'ce0e2e1b39cef330e8d645ddec8724f737f2f44b7c9f4f78dc3b33d62de003cd'

        nodes[0].addMemPoolData(nodes[0].config.privKey, nodes[0].config.pubKey, value, key);

        setTimeout(() => {

            nodes.filter(n => {
                let data = n.getMemPoolData(key)
                return data && data.value == value
            }).length.should.equal(0);
            done();

        }, 1000)
    })


    // let mempool = new Mempool()

    // it('should open the mempool', function() {
    //     // mempool.open();
    // });
    // it('should handle incomming relevant object', function() {

    // });
    // it('should verify invalid received object', function() {

    // });
    // it('should handle duplicate received data', function() {

    // });
    // it('should be able to retrieve a specific data', function() {

    // });
    // it('should destroy the mempool', function() {
    //     // mempool.close();
    // });
});