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

    it('should sync a value on the list of masternodes', function(done) {
        const key = 'testKey';
        const value = new Date().getTime();
        var parms = {}; //use default values
        var nodes = [];

        mocks.mnList.map(mn => {
            nodes.push(new Node(parms));
        })

        nodes[0].addMemPoolData(value, key, 'validPK', 'validSignature');

        setTimeout(() => {
            nodes.filter(n => {
                return n.getMemPoolData(key) == value;
            }).length.should.equal(mocks.mnList.length);
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