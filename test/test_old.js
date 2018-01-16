// These tests were used in mappum's blockchain-spv
// might want to reference to check test scenarios for dash-spv lib

const test = require('tape');
const u = require('dash-util');
const levelup = require('levelup');
const memdown = require('memdown');
const params = require('webcoin-dash').blockchain;
const Blockchain = require('../lib/blockchain.js');
const utils = require('../test/utils.js');

function deleteStore(store, cb) {
  memdown.clearGlobalStore();
  cb();
}

function endStore(store, t) {
  store.close((err) => {
    t.error(err);
    deleteStore(store, t.end);
  });
}

test('create blockchain instance', (t) => {
  t.test('no params', (t) => {
    const db = levelup(`${Math.random()}.chain`, { db: memdown });
    try {
      const chain = new Blockchain(null, db);
      t.notOk(chain, 'should have thrown error');
    } catch (err) {
      t.ok(err, 'threw error');
      t.equal(err.message, 'Invalid blockchain parameters');
      t.end();
    }
  });

  t.test('invalid params', (t) => {
    const db = levelup(`${Math.random()}.chain`, { db: memdown });
    try {
      const chain = new Blockchain({
        genesisHeader: 1,
        shouldRetarget: 1,
        calculateTarget: 1,
        miningHash: 1,
      }, db);
      t.notOk(chain, 'should have thrown error');
    } catch (err) {
      t.ok(err, 'threw error');
      t.equal(err.message, 'Invalid blockchain parameters');
      t.end();
    }
  });

  t.test('no db', (t) => {
    try {
      const chain = new Blockchain(params);
      t.notOk(chain, 'should have thrown error');
    } catch (err) {
      t.ok(err, 'threw error');
      t.equal(err.message, 'Must specify db');
      t.end();
    }
  });

  t.test('valid', (t) => {
    const db = levelup(`${Math.random()}.chain`, { db: memdown });
    const chain = new Blockchain(params, db);
    chain.once('ready', () => {
      endStore(chain.store, t);
    });
  });

  const db = levelup(`${Math.random()}.chain`, { db: memdown });
  const chain = new Blockchain(params, db);

  t.test('before ready', (t) => {
    t.notOk(chain.ready, 'chain.ready === false');
    chain.onceReady(() => { t.end(); });
  });

  t.test('after ready', (t) => {
    t.ok(chain.ready, 'chain.ready === true');
    chain.onceReady(() => { t.end(); });
  });

  t.end();
});


test('close', (t) => {
  const db = levelup(`${Math.random()}.chain`, { db: memdown });
  const chain = new Blockchain(params, db);
  chain.once('ready', () => {
    chain.getBlock(chain.tip.hash, (err, block) => {
      t.error(err, 'no error');
      t.ok('block', 'got block');
      chain.close((err) => {
        t.pass('close cb called');
        t.equal(chain.closed, true, 'chain.closed === true');
        t.error(err, 'no error');
        chain.getBlock(chain.tip.hash, (err, block) => {
          t.ok(err, 'can\'t get from blockstore after chain closed');
          t.equal(err.message, 'Database is not open', 'correct error message');
          t.end();
        });
      });
    });
  });
});

test('getTip', (t) => {
  const db = levelup(`${Math.random()}.chain`, { db: memdown });
  const chain = new Blockchain(params, db);
  chain.once('ready', () => {
    chain.getTip()
      .then((tip) => {
        t.deepEqual(tip, chain.tip, 'got tip');
        t.end();
      });
  });
});

test('blockchain paths', (t) => {
  const testParams = utils.createTestParams({
    genesisHeader: {
      version: 1,
      prevHash: u.nullHash,
      merkleRoot: u.nullHash,
      time: Math.floor(Date.now() / 1000),
      bits: u.compressTarget(utils.maxTarget),
      nonce: 0,
    },
  });
  const genesis = utils.blockFromObject(testParams.genesisHeader);
  const db = levelup('paths.chain', { db: memdown });
  let chain;

  t.test('setup chain', (t) => {
    chain = new Blockchain(testParams, db);
    chain.once('ready', t.end);
  });

  const headers = [];
  t.test('headers add to blockchain', (t) => {
    t.plan(75);
    let block = genesis;
    for (let i = 0; i < 10; i++) {
      block = utils.createBlock(block);
      headers.push(block);
      (function (block) {
        chain.on(`block:${block._getHash().toString('base64')}`, (block2) => {
          t.equal(block, block2.header);
        });
      }(block));
    }

    let blockIndex = 0;
    chain.on('block', (block) => {
      t.equal(block.height, blockIndex + 1);
      t.equal(block.header, headers[blockIndex++]);
    });

    let tipIndex = 0;
    chain.on('tip', (block) => {
      t.equal(block.height, tipIndex + 1);
      t.equal(block.header, headers[tipIndex++]);
    });

    chain.once('headers', (headers2) => {
      t.equal(headers2, headers);
      chain.getBlock(chain.genesis.hash, (err, block) => {
        t.error(err, 'no error');
        t.deepEqual(block.next, headers[0]._getHash(), 'genesis has correct "next"');
      });
      for (let i = 0; i < headers.length - 1; i++) {
        (function (i) {
          chain.getBlock(headers[i]._getHash(), (err, block) => {
            t.error(err, 'no error');
            t.deepEqual(block.next, headers[i + 1]._getHash(), 'block has correct "next"');
          });
        }(i));
      }
      chain.getBlock(headers[9]._getHash(), (err, block) => {
        t.error(err, 'no error');
        t.notOk(block.next, 'block has no "next"');
      });
    });
    chain.addHeaders(headers, (err) => {
      t.pass('addHeaders cb called');
      t.error(err);
    });
  });

  t.test('remove listeners', (t) => {
    chain.removeAllListeners('block');
    chain.removeAllListeners('tip');
    chain.removeAllListeners('blocks');
    t.end();
  });

  t.test('simple path with no fork', (t) => {
    const from = { height: 2, header: headers[1] };
    const to = { height: 10, header: headers[9] };
    chain.getPath(from, to, (err, path) => {
      if (err) return t.end(err);
      t.ok(path);
      t.ok(path.add);
      t.ok(path.remove);
      t.notOk(path.fork);
      t.equal(path.add.length, 8);
      t.equal(path.add[0].height, 3);
      t.equal(path.add[0].header.getId(), headers[2].getId());
      t.equal(path.add[7].height, 10);
      t.equal(path.add[7].header.getId(), to.header.getId());
      t.equal(path.remove.length, 0);
      t.end();
    });
  });

  t.test('backwards path with no fork', (t) => {
    const from = { height: 10, header: headers[9] };
    const to = { height: 2, header: headers[1] };
    chain.getPath(from, to, (err, path) => {
      if (err) return t.end(err);
      t.ok(path);
      t.ok(path.add);
      t.ok(path.remove);
      t.notOk(path.fork);
      t.equal(path.remove.length, 8);
      t.equal(path.remove[0].height, 10);
      t.equal(path.remove[0].header.getId(), from.header.getId());
      t.equal(path.remove[7].height, 3);
      t.equal(path.remove[7].header.getId(), headers[2].getId());
      t.equal(path.add.length, 0);
      t.end();
    });
  });

  const headers2 = [];
  t.test('fork headers add to blockchain', (t) => {
    let block = headers[4];
    for (let i = 0; i < 6; i++) {
      block = utils.createBlock(block, 0xffffff);
      headers2.push(block);
    }
    chain.on('reorg', (e) => {
      t.ok(e, 'got reorg event');
      t.equal(e.path.remove.length, 5, 'removed blocks is correct length');
      t.equal(e.path.remove[0].height, 10);
      t.equal(e.path.remove[0].header.getId(), headers[9].getId());
      t.equal(e.path.remove[1].height, 9);
      t.equal(e.path.remove[1].header.getId(), headers[8].getId());
      t.equal(e.path.remove[2].height, 8);
      t.equal(e.path.remove[2].header.getId(), headers[7].getId());
      t.equal(e.path.remove[3].height, 7);
      t.equal(e.path.remove[3].header.getId(), headers[6].getId());
      t.equal(e.path.remove[4].height, 6);
      t.equal(e.path.remove[4].header.getId(), headers[5].getId());
      t.equal(e.path.add.length, 6, 'added blocks is correct length');
      t.equal(e.path.add[0].height, 6);
      t.equal(e.path.add[0].header.getId(), headers2[0].getId());
      t.deepEqual(e.path.add[0].next, headers2[1]._getHash(), '"next" is correct');
      t.equal(e.path.add[1].height, 7);
      t.equal(e.path.add[1].header.getId(), headers2[1].getId());
      t.deepEqual(e.path.add[1].next, headers2[2]._getHash(), '"next" is correct');
      t.equal(e.path.add[2].height, 8);
      t.equal(e.path.add[2].header.getId(), headers2[2].getId());
      t.deepEqual(e.path.add[2].next, headers2[3]._getHash(), '"next" is correct');
      t.equal(e.path.add[3].height, 9);
      t.equal(e.path.add[3].header.getId(), headers2[3].getId());
      t.deepEqual(e.path.add[3].next, headers2[4]._getHash(), '"next" is correct');
      t.equal(e.path.add[4].height, 10);
      t.equal(e.path.add[4].header.getId(), headers2[4].getId());
      t.deepEqual(e.path.add[4].next, headers2[5]._getHash(), '"next" is correct');
      t.equal(e.path.add[5].height, 11);
      t.equal(e.path.add[5].header.getId(), headers2[5].getId());
      t.notOk(e.path.add[5].next, '"next" is correct');
      t.ok(e.tip);
      t.equal(e.tip.height, 11);
      t.equal(e.tip.header.getId(), headers2[5].getId());
      t.end();
    });
    chain.addHeaders(headers2, t.error);
  });

  t.test('path with fork', (t) => {
    const from = { height: 10, header: headers[9] };
    const to = { height: 11, header: headers2[5] };
    chain.getPath(from, to, (err, path) => {
      if (err) return t.end(err);
      t.ok(path);
      t.ok(path.add);
      t.ok(path.remove);
      t.equal(path.fork.header.getId(), headers[4].getId());
      t.equal(path.remove.length, 5);
      t.equal(path.remove[0].height, 10);
      t.equal(path.remove[0].header.getId(), from.header.getId());
      t.equal(path.remove[4].height, 6);
      t.equal(path.remove[4].header.getId(), headers[5].getId());
      t.equal(path.add.length, 6);
      t.equal(path.add[0].height, 6);
      t.equal(path.add[0].header.getId(), headers2[0].getId());
      t.equal(path.add[5].height, 11);
      t.equal(path.add[5].header.getId(), headers2[5].getId());
      t.end();
    });
  });

  t.test('backwards path with fork', (t) => {
    const from = { height: 11, header: headers2[5] };
    const to = { height: 10, header: headers[9] };
    chain.getPath(from, to, (err, path) => {
      if (err) return t.end(err);
      t.ok(path);
      t.ok(path.add);
      t.ok(path.remove);
      t.equal(path.fork.header.getId(), headers[4].getId());
      t.equal(path.remove.length, 6);
      t.equal(path.remove[0].height, 11);
      t.equal(path.remove[0].header.getId(), from.header.getId());
      t.equal(path.remove[5].height, 6);
      t.equal(path.remove[5].header.getId(), headers2[0].getId());
      t.equal(path.add.length, 5);
      t.equal(path.add[0].height, 6);
      t.equal(path.add[0].header.getId(), headers[5].getId());
      t.equal(path.add[4].height, 10);
      t.equal(path.add[4].header.getId(), headers[9].getId());
      t.end();
    });
  });

  t.test('disjoint path', (t) => {
    const genesis2 = utils.blockFromObject({
      version: 2,
      prevHash: u.nullHash,
      merkleRoot: u.nullHash,
      time: Math.floor(Date.now() / 1000),
      bits: u.compressTarget(utils.maxTarget),
      nonce: 0,
    });
    const from = { height: 0, header: genesis2 };
    const to = chain.tip;
    chain.getPath(from, to, (err, path) => {
      t.ok(err, 'got error');
      t.equal(err.message, 'Blocks are not in the same chain', 'correct error message');
      t.end();
    });
  });

  t.test('getPathToTip', (t) => {
    const from = { height: 10, header: headers[9] };
    chain.getPath(from, chain.tip, (err, path1) => {
      t.error(err);
      chain.getPathToTip(from, (err, path2) => {
        t.error(err);
        t.deepEqual(path1, path2, 'paths are equal');
        t.end();
      });
    });
  });

  t.test('deleting blockstore', (t) => {
    endStore(chain.store, t);
  });
});

test('blockchain verification', (t) => {
  const testParams = utils.createTestParams({
    interval: 10,
  });
  const db = levelup('verification.chain', { db: memdown });
  const chain = new Blockchain(testParams, db);

  const headers = [];
  const genesis = utils.blockFromObject(testParams.genesisHeader);
  t.test('headers add to blockchain', (t) => {
    let block = genesis;
    for (let i = 0; i < 9; i++) {
      block = utils.createBlock(block);
      headers.push(block);
    }
    chain.addHeaders(headers, t.end);
  });

  t.test('error on header that doesn\'t connect', (t) => {
    const block = utils.createBlock();
    chain.addHeaders([block], (err) => {
      t.ok(err);
      t.equal(err.message, 'Block does not connect to chain');
      t.end();
    });
  });

  t.test('error on nonconsecutive headers', (t) => {
    const block1 = utils.createBlock(headers[5], 10000);
    const block2 = utils.createBlock(headers[6], 10000);

    chain.addHeaders([block1, block2], (err) => {
      t.ok(err);
      t.equal(err.message, 'Block does not connect to previous');
      t.end();
    });
  });

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

  t.test('teardown', (t) => {
    endStore(chain.store, t);
  });
});

test('blockchain queries', (t) => {
  const testParams = utils.createTestParams();
  const genesis = utils.blockFromObject(testParams.genesisHeader);
  const db = levelup('queries.chain', { db: memdown });
  const chain = new Blockchain(testParams, db);

  const headers = [];
  t.test('setup', (t) => {
    let block = genesis;
    for (let i = 0; i < 100; i++) {
      block = utils.createBlock(block);
      headers.push(block);
    }
    chain.addHeaders(headers, t.end);
  });

  t.test('get block at height', (t) => {
    t.plan(14);

    chain.getBlockAtHeight(10, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 10);
      t.equal(block.header.getId(), headers[9].getId());
    });

    chain.getBlockAtHeight(90, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 90);
      t.equal(block.header.getId(), headers[89].getId());
    });

    chain.getBlockAtHeight(200, (err, block) => {
      t.ok(err);
      t.notOk(block);
      t.equal(err.message, 'height is higher than tip');
    });

    chain.getBlockAtHeight(-10, (err, block) => {
      t.ok(err);
      t.notOk(block);
      t.equal(err.message, 'height must be >= 0');
    });
  });

  t.test('get block at time', (t) => {
    t.plan(16);

    chain.getBlockAtTime(genesis.time + 9, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 10);
      t.equal(block.header.getId(), headers[9].getId());
    });

    chain.getBlockAtTime(genesis.time + 89, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 90);
      t.equal(block.header.getId(), headers[89].getId());
    });

    chain.getBlockAtTime(genesis.time + 200, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 100);
      t.equal(block.header.getId(), headers[99].getId());
    });

    chain.getBlockAtTime(genesis.time - 10, (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 0);
      t.equal(block.header.getId(), genesis.getId());
    });
  });

  t.test('get block by hash', (t) => {
    t.plan(6);

    chain.getBlock(headers[50]._getHash(), (err, block) => {
      t.error(err);
      t.ok(block);
      t.equal(block.height, 51);
      t.equal(block.header.getId(), headers[50].getId());
    });

    chain.getBlock(123, (err, block) => {
      t.ok(err);
      t.equal(err.message, '"hash" must be a Buffer');
    });
  });

  t.test('get locator', (t) => {
    chain.getLocator((err, locator) => {
      t.error(err, 'no error');
      t.ok(locator, 'got locator');
      t.equal(locator.length, 6, 'locator has 6 hashes');
      t.deepEqual(locator[0], headers[99]._getHash());
      t.deepEqual(locator[1], headers[98]._getHash());
      t.deepEqual(locator[2], headers[97]._getHash());
      t.deepEqual(locator[3], headers[96]._getHash());
      t.deepEqual(locator[4], headers[95]._getHash());
      t.deepEqual(locator[5], headers[94]._getHash());
      t.end();
    });
  });

  t.test('teardown', (t) => {
    endStore(chain.store, t);
  });
});

test('streams', (t) => {
  const testParams = utils.createTestParams({
    genesisHeader: {
      version: 1,
      prevHash: u.nullHash,
      merkleRoot: u.nullHash,
      time: Math.floor(Date.now() / 1000),
      bits: u.compressTarget(utils.maxTarget),
      nonce: 0,
    },
  });
  const genesis = utils.blockFromObject(testParams.genesisHeader);
  const db = levelup('streams.chain', { db: memdown });
  const chain = new Blockchain(testParams, db);
  let writeStream;

  t.test('wait for ready', (t) => {
    chain.onceReady(t.end.bind(t));
  });

  t.test('write stream', (t) => {
    writeStream = chain.createWriteStream();
    t.ok(writeStream, 'got writeStream');
    t.ok(writeStream.writable, 'is writable');
    let prev = genesis;
    const headers = [];
    for (let i = 0; i < 10; i++) {
      prev = headers[i] = utils.createBlock(prev);
    }
    chain.once('consumed', () => {
      t.equal(chain.tip.height, 10, 'chain now has higher tip');
      t.deepEqual(chain.tip.header, headers[9], 'chain tip has correct header');
      t.end();
    });
    writeStream.write(headers);
  });

  t.test('read stream', (t) => {
    // see headerStream.js for more detailed tests
    t.plan(23);
    const readStream = chain.createReadStream();
    t.ok(readStream, 'got readStream');
    let height = 0;
    readStream.on('data', (block) => {
      t.ok(block, 'got block');
      t.equal(block.height, height++, 'block at correct height');
      if (block.height === 10) readStream.end();
    });
  });

  t.test('locator stream', (t) => {
    let locatorStream;
    t.test('create locator stream', (t) => {
      locatorStream = chain.createLocatorStream();
      t.ok(locatorStream, 'got locator stream');
      t.end();
    });

    t.test('get initial locator', (t) => {
      locatorStream.once('data', (locator) => {
        t.ok(locator, 'got locator');
        t.equal(locator.length, 6, 'locator is correct length');
        t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip');
        t.notOk(locatorStream.read(), 'nothing left to read');
        t.end();
      });
    });

    t.test('locator pushed after valid headers added', (t) => {
      locatorStream.once('data', (locator) => {
        t.ok(locator, 'got locator');
        t.equal(locator.length, 6, 'locator is correct length');
        t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip');
        t.notOk(locatorStream.read(), 'nothing left to read');
        t.end();
      });
      let prev = chain.tip.header;
      const headers = [];
      for (let i = 0; i < 10; i++) {
        prev = headers[i] = utils.createBlock(prev);
      }
      chain.addHeaders(headers, () => { });
    });

    t.test('locator pushed after invalid headers added', (t) => {
      locatorStream.once('data', (locator) => {
        t.ok(locator, 'got locator');
        t.equal(locator.length, 6, 'locator is correct length');
        t.deepEqual(locator[0], chain.tip.hash, 'locator starts with tip');
        t.notOk(locatorStream.read(), 'nothing left to read');
        t.end();
      });
      const genesis2 = utils.blockFromObject({
        version: 2,
        prevHash: u.nullHash,
        merkleRoot: u.nullHash,
        time: Math.floor(Date.now() / 1000),
        bits: u.compressTarget(utils.maxTarget),
        nonce: 0,
      });
      chain.addHeaders([genesis2], () => { });
    });

    t.end();
  });
});
