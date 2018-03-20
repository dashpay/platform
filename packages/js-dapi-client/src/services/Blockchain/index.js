const { restore } = require('./restore');
const normalizeHeader = require('./util/_normalizeHeader');
const { init } = require('./init');
const { expectNextDifficulty } = require('./expectNextDifficulty');
const { addBlock } = require('./addBlock');
const { getBlock } = require('./getBlock');
const { getLastBlock } = require('./getLastBlock');

// TODO: Fix dangling comma use
const Blockchain = () => ({
  restore,
  normalizeHeader,
  init,
  expectNextDifficulty,
  addBlock,
  getBlock,
  getLastBlock,
});

module.exports = { Blockchain };
