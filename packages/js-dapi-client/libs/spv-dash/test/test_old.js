//These tests were used in mappum's blockchain-spv
//might want to reference to check test scenarios for dash-spv lib 

var test = require('tape')
var u = require('dash-util')
var levelup = require('levelup')
var memdown = require('memdown')
var params = require('webcoin-dash').blockchain
var Blockchain = require('../lib/blockchain.js')
var utils = require('../test/utils.js')

function deleteStore(store, cb) {
    memdown.clearGlobalStore()
    cb()
}

function endStore(store, t) {
    store.close(function(err) {
        t.error(err)
        deleteStore(store, t.end)
    })
}

test('create blockchain instance', function(t) {
    t.test('no params', function(t) {
        var db = levelup(Math.random() + '.chain', { db: memdown })
        try {
            var chain = new Blockchain(null, db)
            t.notOk(chain, 'should have thrown error')
        } catch (err) {
            t.ok(err, 'threw error')
            t.equal(err.message, 'Invalid blockchain parameters')
            t.end()
        }
    })

    t.test('invalid params', function(t) {
        var db = levelup(Math.random() + '.chain', { db: memdown })
        try {
            var chain = new Blockchain({
                genesisHeader: 1,
                shouldRetarget: 1,
                calculateTarget: 1,
                miningHash: 1
            }, db)
            t.notOk(chain, 'should have thrown error')
        } catch (err) {
            t.ok(err, 'threw error')
            t.equal(err.message, 'Invalid blockchain parameters')
            t.end()
        }
    })

    t.test('no db', function(t) {
        try {
            var chain = new Blockchain(params)
            t.notOk(chain, 'should have thrown error')
        } catch (err) {
            t.ok(err, 'threw error')
            t.equal(err.message, 'Must specify db')
            t.end()
        }
    })

    t.test('valid', function(t) {
        var db = levelup(Math.random() + '.chain', { db: memdown })
        var chain = new Blockchain(params, db)
        chain.once('ready', function() {
            endStore(chain.store, t)
        })
    })

    var db = levelup(Math.random() + '.chain', { db: memdown })
    var chain = new Blockchain(params, db)

    t.test('before ready', function(t) {
        t.notOk(chain.ready, 'chain.ready === false')
        chain.onceReady(function() { t.end() })
    })

    t.test('after ready', function(t) {
        t.ok(chain.ready, 'chain.ready === true')
        chain.onceReady(function() { t.end() })
    })

    t.end()
})


test('close', function(t) {
    var db = levelup(Math.random() + '.chain', { db: memdown })
    var chain = new Blockchain(params, db)
    chain.once('ready', function() {
        chain.getBlock(chain.tip.hash, function(err, block) {
            t.error(err, 'no error')
            t.ok('block', 'got block')
            chain.close(function(err) {
                t.pass('close cb called')
                t.equal(chain.closed, true, 'chain.closed === true')
                t.error(err, 'no error')
                chain.getBlock(chain.tip.hash, function(err, block) {
                    t.ok(err, 'can\'t get from blockstore after chain closed')
                    t.equal(err.message, 'Database is not open', 'correct error message')
                    t.end()
                })
            })
        })
    })
})

test('getTip', function(t) {
    var db = levelup(Math.random() + '.chain', { db: memdown })
    var chain = new Blockchain(params, db)
    chain.once('ready', function() {
        chain.getTip()
            .then(tip => {
                t.deepEqual(tip, chain.tip, 'got tip')
                t.end()
            })
    })
})

test('blockchain paths', function(t) {
    var testParams = utils.createTestParams({
        genesisHeader: {
            version: 1,
            prevHash: u.nullHash,
            merkleRoot: u.nullHash,
            time: Math.floor(Date.now() / 1000),
            bits: u.compressTarget(utils.maxTarget),
            nonce: 0
        }
    })
    var genesis = utils.blockFromObject(testParams.genesisHeader)
    var db = levelup('paths.chain', { db: memdown })
    var chain

    t.test('setup chain', function(t) {
        chain = new Blockchain(testParams, db)
        chain.once('ready', t.end)
    })

    var headers = []
    t.test('headers add to blockchain', function(t) {
        t.plan(75)
        var block = genesis
        for (var i = 0; i < 10; i++) {
            block = utils.createBlock(block)
            headers.push(block);
            (function(block) {
                chain.on('block:' + block._getHash().toString('base64'), function(block2) {
                    t.equal(block, block2.header)
                })
            })(block)
        }

        var blockIndex = 0
        chain.on('block', function(block) {
            t.equal(block.height, blockIndex + 1)
            t.equal(block.header, headers[blockIndex++])
        })

        var tipIndex = 0
        chain.on('tip', function(block) {
            t.equal(block.height, tipIndex + 1)
            t.equal(block.header, headers[tipIndex++])
        })

        chain.once('headers', function(headers2) {
            t.equal(headers2, headers)
            chain.getBlock(chain.genesis.hash, function(err, block) {
                t.error(err, 'no error')
                t.deepEqual(block.next, headers[0]._getHash(), 'genesis has correct "next"')
            })
            for (var i = 0; i < headers.length - 1; i++) {
                (function(i) {
                    chain.getBlock(headers[i]._getHash(), function(err, block) {
                        t.error(err, 'no error')
                        t.deepEqual(block.next, headers[i + 1]._getHash(), 'block has correct "next"')
                    })
                })(i)
            }
            chain.getBlock(headers[9]._getHash(), function(err, block) {
                t.error(err, 'no error')
                t.notOk(block.next, 'block has no "next"')
            })
        })
        chain.addHeaders(headers, function(err) {
            t.pass('addHeaders cb called')
            t.error(err)
        })
    })

    t.test('remove listeners', function(t) {
        chain.removeAllListeners('block')
        chain.removeAllListeners('tip')
        chain.removeAllListeners('blocks')
        t.end()
    })

    t.test('simple path with no fork', function(t) {
        var from = { height: 2, header: headers[1] }
        var to = { height: 10, header: headers[9] }
        chain.getPath(from, to, function(err, path) {
            if (err) return t.end(err)
            t.ok(path)
            t.ok(path.add)
            t.ok(path.remove)
            t.notOk(path.fork)
            t.equal(path.add.length, 8)
            t.equal(path.add[0].height, 3)
            t.equal(path.add[0].header.getId(), headers[2].getId())
            t.equal(path.add[7].height, 10)
            t.equal(path.add[7].header.getId(), to.header.getId())
            t.equal(path.remove.length, 0)
            t.end()
        })
    })

    t.test('backwards path with no fork', function(t) {
        var from = { height: 10, header: headers[9] }
        var to = { height: 2, header: headers[1] }
        chain.getPath(from, to, function(err, path) {
            if (err) return t.end(err)
            t.ok(path)
            t.ok(path.add)
            t.ok(path.remove)
            t.notOk(path.fork)
            t.equal(path.remove.length, 8)
            t.equal(path.remove[0].height, 10)
            t.equal(path.remove[0].header.getId(), from.header.getId())
            t.equal(path.remove[7].height, 3)
            t.equal(path.remove[7].header.getId(), headers[2].getId())
            t.equal(path.add.length, 0)
            t.end()
        })
    })

    var headers2 = []
    t.test('fork headers add to blockchain', function(t) {
        var block = headers[4]
        for (var i = 0; i < 6; i++) {
            block = utils.createBlock(block, 0xffffff)
            headers2.push(block)
        }
        chain.on('reorg', function(e) {
            t.ok(e, 'got reorg event')
            t.equal(e.path.remove.length, 5, 'removed blocks is correct length')
            t.equal(e.path.remove[0].height, 10)
            t.equal(e.path.remove[0].header.getId(), headers[9].getId())
            t.equal(e.path.remove[1].height, 9)
            t.equal(e.path.remove[1].header.getId(), headers[8].getId())
            t.equal(e.path.remove[2].height, 8)
            t.equal(e.path.remove[2].header.getId(), headers[7].getId())
            t.equal(e.path.remove[3].height, 7)
            t.equal(e.path.remove[3].header.getId(), headers[6].getId())
            t.equal(e.path.remove[4].height, 6)
            t.equal(e.path.remove[4].header.getId(), headers[5].getId())
            t.equal(e.path.add.length, 6, 'added blocks is correct length')
            t.equal(e.path.add[0].height, 6)
            t.equal(e.path.add[0].header.getId(), headers2[0].getId())
            t.deepEqual(e.path.add[0].next, headers2[1]._getHash(), '"next" is correct')
            t.equal(e.path.add[1].height, 7)
            t.equal(e.path.add[1].header.getId(), headers2[1].getId())
            t.deepEqual(e.path.add[1].next, headers2[2]._getHash(), '"next" is correct')
            t.equal(e.path.add[2].height, 8)
            t.equal(e.path.add[2].header.getId(), headers2[2].getId())
            t.deepEqual(e.path.add[2].next, headers2[3]._getHash(), '"next" is correct')
            t.equal(e.path.add[3].height, 9)
            t.equal(e.path.add[3].header.getId(), headers2[3].getId())
            t.deepEqual(e.path.add[3].next, headers2[4]._getHash(), '"next" is correct')
            t.equal(e.path.add[4].height, 10)
            t.equal(e.path.add[4].header.getId(), headers2[4].getId())
            t.deepEqual(e.path.add[4].next, headers2[5]._getHash(), '"next" is correct')
            t.equal(e.path.add[5].height, 11)
            t.equal(e.path.add[5].header.getId(), headers2[5].getId())
            t.notOk(e.path.add[5].next, '"next" is correct')
            t.ok(e.tip)
            t.equal(e.tip.height, 11)
            t.equal(e.tip.header.getId(), headers2[5].getId())
            t.end()
        })
        chain.addHeaders(headers2, t.error)
    })

    t.test('path with fork', function(t) {
        var from = { height: 10, header: headers[9] }
        var to = { height: 11, header: headers2[5] }
        chain.getPath(from, to, function(err, path) {
            if (err) return t.end(err)
            t.ok(path)
            t.ok(path.add)
            t.ok(path.remove)
            t.equal(path.fork.header.getId(), headers[4].getId())
            t.equal(path.remove.length, 5)
            t.equal(path.remove[0].height, 10)
            t.equal(path.remove[0].header.getId(), from.header.getId())
            t.equal(path.remove[4].height, 6)
            t.equal(path.remove[4].header.getId(), headers[5].getId())
            t.equal(path.add.length, 6)
            t.equal(path.add[0].height, 6)
            t.equal(path.add[0].header.getId(), headers2[0].getId())
            t.equal(path.add[5].height, 11)
            t.equal(path.add[5].header.getId(), headers2[5].getId())
            t.end()
        })
    })

    t.test('backwards path with fork', function(t) {
        var from = { height: 11, header: headers2[5] }
        var to = { height: 10, header: headers[9] }
        chain.getPath(from, to, function(err, path) {
            if (err) return t.end(err)
            t.ok(path)
            t.ok(path.add)
            t.ok(path.remove)
            t.equal(path.fork.header.getId(), headers[4].getId())
            t.equal(path.remove.length, 6)
            t.equal(path.remove[0].height, 11)
            t.equal(path.remove[0].header.getId(), from.header.getId())
            t.equal(path.remove[5].height, 6)
            t.equal(path.remove[5].header.getId(), headers2[0].getId())
            t.equal(path.add.length, 5)
            t.equal(path.add[0].height, 6)
            t.equal(path.add[0].header.getId(), headers[5].getId())
            t.equal(path.add[4].height, 10)
            t.equal(path.add[4].header.getId(), headers[9].getId())
            t.end()
        })
    })

    t.test('disjoint path', function(t) {
        var genesis2 = utils.blockFromObject({
            version: 2,
            prevHash: u.nullHash,
            merkleRoot: u.nullHash,
            time: Math.floor(Date.now() / 1000),
            bits: u.compressTarget(utils.maxTarget),
            nonce: 0
        })
        var from = { height: 0, header: genesis2 }
        var to = chain.tip
        chain.getPath(from, to, function(err, path) {
            t.ok(err, 'got error')
            t.equal(err.message, 'Blocks are not in the same chain', 'correct error message')
            t.end()
        })
    })

    t.test('getPathToTip', function(t) {
        var from = { height: 10, header: headers[9] }
        chain.getPath(from, chain.tip, function(err, path1) {
            t.error(err)
            chain.getPathToTip(from, function(err, path2) {
                t.error(err)
                t.deepEqual(path1, path2, 'paths are equal')
                t.end()
            })
        })
    })

    t.test('deleting blockstore', function(t) {
        endStore(chain.store, t)
    })
})

test('blockchain verification', function(t) {
    var testParams = utils.createTestParams({
        interval: 10
    })
    var db = levelup('verification.chain', { db: memdown })
    var chain = new Blockchain(testParams, db)

    var headers = []
    var genesis = utils.blockFromObject(testParams.genesisHeader)
    t.test('headers add to blockchain', function(t) {
        var block = genesis
        for (var i = 0; i < 9; i++) {
            block = utils.createBlock(block)
            headers.push(block)
        }
        chain.addHeaders(headers, t.end)
    })

    t.test('error on header that doesn\'t connect', function(t) {
        var block = utils.createBlock()
        chain.addHeaders([block], function(err) {
            t.ok(err)
            t.equal(err.message, 'Block does not connect to chain')
            t.end()
        })
    })

    t.test('error on nonconsecutive headers', function(t) {
        var block1 = utils.createBlock(headers[5], 10000)
        var block2 = utils.createBlock(headers[6], 10000)

        chain.addHeaders([block1, block2], function(err) {
            t.ok(err)
            t.equal(err.message, 'Block does not connect to previous')
            t.end()
        })
    })

    /*
    // TODO: establish test coverage for SPV
  
    t.test('error on header with unexpected difficulty change', function (t) {
      var block = utils.createBlock(headers[5])
      block.bits = 0x1d00ffff
      chain.addHeaders([ block ], function (err) {
        t.ok(err)
        t.equal(err.message, 'Unexpected difficulty change at height 7')
        t.end()
      })
    })
  
    t.test('error on header with invalid proof of work', function (t) {
      var block = utils.createBlock(headers[8], 0, genesis.bits, false)
      chain.addHeaders([ block ], function (err) {
        t.ok(err)
        t.ok(err.message.indexOf('Mining hash is above target') === 0)
        t.end()
      })
    })
  
    t.test('error on header with invalid difficulty change', function (t) {
      var block = utils.createBlock(headers[8], 0, 0x207fffff)
      chain.addHeaders([ block ], function (err) {
        t.ok(err)
        t.equal(err.message, 'Bits in block (207fffff) different than expected (201fffff)')
        t.end()
      })
    })
  
    t.test('accept valid difficulty change', function (t) {
      var block = utils.createBlock(headers[8], 0, 0x201fffff)
      chain.addHeaders([ block ], t.end)
    })
    */

    t.test('teardown', function(t) {
        endStore(chain.store, t)
    })
})

test('blockchain queries', function(t) {
    var testParams = utils.createTestParams()
    var genesis = utils.blockFromObject(testParams.genesisHeader)
    var db = levelup('queries.chain', { db: memdown })
    var chain = new Blockchain(testParams, db)

    var headers = []
    t.test('setup', function(t) {
        var block = genesis
        for (var i = 0; i < 100; i++) {
            block = utils.createBlock(block)
            headers.push(block)
        }
        chain.addHeaders(headers, t.end)
    })

    t.test('get block at height', function(t) {
        t.plan(14)

        chain.getBlockAtHeight(10, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 10)
            t.equal(block.header.getId(), headers[9].getId())
        })

        chain.getBlockAtHeight(90, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 90)
            t.equal(block.header.getId(), headers[89].getId())
        })

        chain.getBlockAtHeight(200, function(err, block) {
            t.ok(err)
            t.notOk(block)
            t.equal(err.message, 'height is higher than tip')
        })

        chain.getBlockAtHeight(-10, function(err, block) {
            t.ok(err)
            t.notOk(block)
            t.equal(err.message, 'height must be >= 0')
        })
    })

    t.test('get block at time', function(t) {
        t.plan(16)

        chain.getBlockAtTime(genesis.time + 9, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 10)
            t.equal(block.header.getId(), headers[9].getId())
        })

        chain.getBlockAtTime(genesis.time + 89, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 90)
            t.equal(block.header.getId(), headers[89].getId())
        })

        chain.getBlockAtTime(genesis.time + 200, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 100)
            t.equal(block.header.getId(), headers[99].getId())
        })

        chain.getBlockAtTime(genesis.time - 10, function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 0)
            t.equal(block.header.getId(), genesis.getId())
        })
    })

    t.test('get block by hash', function(t) {
        t.plan(6)

        chain.getBlock(headers[50]._getHash(), function(err, block) {
            t.error(err)
            t.ok(block)
            t.equal(block.height, 51)
            t.equal(block.header.getId(), headers[50].getId())
        })

        chain.getBlock(123, function(err, block) {
            t.ok(err)
            t.equal(err.message, '"hash" must be a Buffer')
        })
    })

    t.test('get locator', function(t) {
        chain.getLocator(function(err, locator) {
            t.error(err, 'no error')
            t.ok(locator, 'got locator')
            t.equal(locator.length, 6, 'locator has 6 hashes')
            t.deepEqual(locator[0], headers[99]._getHash())
            t.deepEqual(locator[1], headers[98]._getHash())
            t.deepEqual(locator[2], headers[97]._getHash())
            t.deepEqual(locator[3], headers[96]._getHash())
            t.deepEqual(locator[4], headers[95]._getHash())
            t.deepEqual(locator[5], headers[94]._getHash())
            t.end()
        })
    })

    t.test('teardown', function(t) {
        endStore(chain.store, t)
    })
})

test('streams', function(t) {
    var testParams = utils.createTestParams({
        genesisHeader: {
            version: 1,
            prevHash: u.nullHash,
            merkleRoot: u.nullHash,
            time: Math.floor(Date.now() / 1000),
            bits: u.compressTarget(utils.maxTarget),
            nonce: 0
        }
    })
    var genesis = utils.blockFromObject(testParams.genesisHeader)
    var db = levelup('streams.chain', { db: memdown })
    var chain = new Blockchain(testParams, db)
    var writeStream

    t.test('wait for ready', function(t) {
        chain.onceReady(t.end.bind(t))
    })

    t.test('write stream', function(t) {
        writeStream = chain.createWriteStream()
        t.ok(writeStream, 'got writeStream')
        t.ok(writeStream.writable, 'is writable')
        var prev = genesis
        var headers = []
        for (var i = 0; i < 10; i++) {
            prev = headers[i] = utils.createBlock(prev)
        }
        chain.once('consumed', function() {
            t.equal(chain.tip.height, 10, 'chain now has higher tip')
            t.deepEqual(chain.tip.header, headers[9], 'chain tip has correct header')
            t.end()
        })
        writeStream.write(headers)
    })

    t.test('read stream', function(t) {
        // see headerStream.js for more detailed tests
        t.plan(23)
        var readStream = chain.createReadStream()
        t.ok(readStream, 'got readStream')
        var height = 0
        readStream.on('data', function(block) {
            t.ok(block, 'got block')
            t.equal(block.height, height++, 'block at correct height')
            if (block.height === 10) readStream.end()
        })
    })

    t.test('locator stream', function(t) {
        var locatorStream
        t.test('create locator stream', function(t) {
            locatorStream = chain.createLocatorStream()
            t.ok(locatorStream, 'got locator stream')
            t.end()
        })

        t.test('get initial locator', function(t) {
            locatorStream.once('data', function(locator) {
                t.ok(locator, 'got locator')
                t.equal(locator.length, 6, 'locator is correct length')
                t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip')
                t.notOk(locatorStream.read(), 'nothing left to read')
                t.end()
            })
        })

        t.test('locator pushed after valid headers added', function(t) {
            locatorStream.once('data', function(locator) {
                t.ok(locator, 'got locator')
                t.equal(locator.length, 6, 'locator is correct length')
                t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip')
                t.notOk(locatorStream.read(), 'nothing left to read')
                t.end()
            })
            var prev = chain.tip.header
            var headers = []
            for (var i = 0; i < 10; i++) {
                prev = headers[i] = utils.createBlock(prev)
            }
            chain.addHeaders(headers, function() { })
        })

        t.test('locator pushed after invalid headers added', function(t) {
            locatorStream.once('data', function(locator) {
                t.ok(locator, 'got locator')
                t.equal(locator.length, 6, 'locator is correct length')
                t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip')
                t.notOk(locatorStream.read(), 'nothing left to read')
                t.end()
            })
            var genesis2 = utils.blockFromObject({
                version: 2,
                prevHash: u.nullHash,
                merkleRoot: u.nullHash,
                time: Math.floor(Date.now() / 1000),
                bits: u.compressTarget(utils.maxTarget),
                nonce: 0
            })
            chain.addHeaders([genesis2], function() { })
        })

        t.end()
    })
})
