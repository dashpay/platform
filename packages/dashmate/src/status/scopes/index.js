const host = require('./host');
const core = require('./core');
const masternode = require('./masternode');
const platform = require('./platform');
const services = require('./services');
const overview = require('./overview');

/**
 *
 * @type {{host: Function,core: Function, platform:Function,services:Function}}
 */
module.exports = {
  host, core, masternode, platform, services, overview,
};
