const os = require("os");
const publicIp = require('public-ip');
const prettyMs = require('pretty-ms');
const prettyByte = require('pretty-bytes');

module.exports = async () => ({
  hostname: os.hostname(),
  uptime: prettyMs(os.uptime() * 1000),
  platform: os.platform(),
  arch: os.arch(),
  username: os.userInfo().username,
  diskFree: 0, // Waiting for feature: https://github.com/nodejs/node/pull/31351
  memory: `${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`,
  cpus: os.cpus().length,
  ip: await publicIp.v4(),
})
