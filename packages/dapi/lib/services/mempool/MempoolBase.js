const ipfsapi = require('ipfs-api');
const DapiDB = require('@dashevo/dapi-db');
// const util = require('util');

class MempoolBase {
  constructor(port = 5001) {
    try {
      this.orbitdb = new DapiDB(ipfsapi('127.0.0.1', port));
    } catch (error) {
      // console.log(`Check if ipfs daemon is running on port ${port}. Exception: ${error}`);
    }
  }

  // dumpObj(obj) {
  //   console.log(util.inspect(
  //     obj,
  //     {
  //       showHidden: true,
  //       depth: null,
  //       maxArrayLength: null,
  //       breakLength: null,
  //     },
  //   ));
  // }
}

module.exports = MempoolBase;
