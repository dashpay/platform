const ipfsapi = require('ipfs-api'),
    OrbitDB = require('orbit-db'),
    util = require("util")

class MempoolBase {
    constructor(port = 5001) {
        this.orbitdb = new OrbitDB(ipfsapi('127.0.0.1', port));
    }

    dump_obj(obj) {
        console.log(
            util.inspect(
                obj,
                {
                    showHidden: true,
                    depth: null,
                    maxArrayLength: null,
                    breakLength: null
                })
        )
        return null;
    }
}

module.exports = MempoolBase