'use strict'
const should = require('should');
const assert = require('assert');
const kvs = require('../../lib/mempool/keyValueStore');

describe('Network - Mempool', function() {

    it('should create 2 nodes and write a value on node 1 and get synced data on node 2', function(done) {
        const value = new Date().getTime();
        let n1 = new kvs();
        let n2 = new kvs();

        n1.writeValue(value);

        setTimeout(() => {
            n2.getValue().should.equal(value);
            done();
        }, 500)

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