const wait = require("../util/wait");
const publicIp = require('public-ip');

async function resolvePublicIpV4() {
  return Promise.race([
    publicIp.v4().catch(() => null),
    // Resolve in 10 seconds if public IP is not available
    wait(10000).then(() => null),
  ])
}

module.exports = resolvePublicIpV4;
